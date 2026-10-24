use super::{
    NotifySender,
    client::{Authenticated, Client},
    config::Config,
    db::{DatabaseClient, decicelsius_to_celsius},
    rate_limit::RateLimiter,
};
use crate::{
    error::Error,
    server::{config::NotificationEventsConfig, db::DatabaseBackend},
};
use pwmp_client::pwmp_msg::{
    request::Request::{self},
    response::Response,
};
use std::{net::SocketAddr, sync::Arc};
use tokio::{net::TcpStream, time::timeout};
use tracing::{debug, error, warn};

/// Maximum OTA chunk size a client can request.
const MAX_OTA_CHUNK_SIZE: u32 = 4 * 1024 * 1024; // 4 MiB

#[allow(clippy::needless_pass_by_value, clippy::cognitive_complexity)]
pub async fn handle_client(
    client: TcpStream,
    peer_addr: SocketAddr,
    db: &DatabaseClient,
    notify: &NotifySender,
    config: Arc<Config>,
) -> Result<(), Error> {
    let client = Client::new(client, peer_addr);
    let mut rate_limiter = RateLimiter::new(config.rate_limits.max_requests);
    let maybe_client = timeout(config.limits.stall_time, client.authorize(db)).await;

    let mut client = match maybe_client {
        // no timeout + auth successfull
        Ok(Ok(client)) => client,

        // error while processing request
        Ok(Err(why)) => return Err(why),

        // Timed out
        Err(..) => {
            error!("{peer_addr}: Stalled for too long, kicking");
            return Err(Error::StallTimeExceeded);
        }
    };

    loop {
        let maybe_request = timeout(config.limits.stall_time, client.receive_request()).await;

        let request = match maybe_request {
            // Successfully received and parsed a request
            Ok(Ok(req)) => req,

            // An error occured while receiving a request
            Ok(Err(why)) => return Err(why),

            // Timed out
            Err(..) => {
                error!("{}: Stalled for too long, kicking", client.id());
                let _ = client.shutdown(Some(Response::Stalling)).await;
                return Err(Error::StallTimeExceeded);
            }
        };

        if rate_limiter.hit() {
            error!("{}: Exceeded request limits", client.id());
            client.shutdown(Some(Response::RateLimitExceeded)).await?;
            break;
        }

        if request == Request::Bye {
            debug!("{}: Bye", client.id());
            client.shutdown(None).await?;
            break;
        }

        match handle_request(
            request,
            &mut client,
            db,
            notify,
            &config.notification.events,
        )
        .await
        {
            Ok(response) => {
                if response.is_error() {
                    error!(
                        "{}: Error while processing request: {response:?}",
                        client.id()
                    );

                    client.shutdown(Some(response)).await?;
                    return Ok(());
                }

                if let Err(why) =
                    timeout(config.limits.stall_time, client.send_response(response)).await
                {
                    error!("{}: Timed out while sending response: {why}", client.id());
                }
            }
            Err(why) => {
                client.shutdown(Some(Response::InternalServerError)).await?;
                return Err(why);
            }
        }
    }

    Ok(())
}

#[allow(clippy::too_many_lines)]
#[tracing::instrument(name = "handle_request()", skip_all, level = "debug", err, ret)]
async fn handle_request(
    req: Request,
    client: &mut Client<Authenticated>,
    db: &DatabaseClient,
    notify: &NotifySender,
    notifs_cfg: &NotificationEventsConfig,
) -> Result<Response, Error> {
    debug!("Handling {req:#?}");

    match req {
        Request::Hello { .. } => {
            error!("Received another handshake message");
            Ok(Response::InvalidRequest)
        }
        Request::PostMeasurements {
            temperature,
            humidity,
            air_pressure,
            battery,
            cpu_temp,
            wifi_ssid,
            wifi_rssi,
        } => {
            if client.last_submit().is_some() {
                error!(
                    "{}: Submitted multiple posts, which is not allowed",
                    client.id()
                );
                return Ok(Response::InvalidRequest);
            }

            let celsius = decicelsius_to_celsius(temperature);

            debug!(
                "{}: {celsius:.02}°C, {humidity}%, {air_pressure:?}hPa",
                client.id()
            );
            db.post_measurements(
                client.id(),
                temperature,
                humidity,
                air_pressure,
                cpu_temp,
                battery,
                &wifi_ssid,
                wifi_rssi,
            )
            .await?;

            if notifs_cfg.on_measurements_posted {
                notify_send(
                    notify,
                    client,
                    db,
                    format!(
                        "{celsius:.02}°C, {humidity}%, {}hPa",
                        air_pressure.map_or_else(|| "-".to_string(), |val| val.to_string())
                    ),
                )
                .await?;
            }

            Ok(Response::Ok)
        }
        Request::PostDiagnosticsArtifact { kind, .. } => {
            warn!(
                "{}: Diagnostic artifacts are not supported yet (got \"{kind}\")",
                client.id()
            );
            Ok(Response::InvalidRequest)
        }
        Request::SendNotification(message) => {
            notify_send(notify, client, db, message).await?;
            Ok(Response::Ok)
        }
        Request::GetSettings => {
            let settings = client.get_settings(db).await?;

            if settings.is_none() {
                warn!("{}: Settings are undefined", client.id());
            }

            Ok(Response::Settings(settings))
        }
        Request::UpdateCheck(current_ver) => {
            debug!("{}: Claims OS version {}", client.id(), current_ver);
            let update_info = db.check_os_update(client.id(), current_ver).await?;

            if let Some((version, firmware_blob)) = update_info {
                client.store_update_check_result(current_ver, version, firmware_blob);

                if notifs_cfg.on_update_discovered {
                    notify_send(
                        notify,
                        client,
                        db,
                        format!("Update {version} available, currently running {current_ver}"),
                    )
                    .await?;
                }

                Ok(Response::UpdateAvailable(version))
            } else {
                client.mark_up_to_date();
                Ok(Response::FirmwareUpToDate)
            }
        }
        Request::NextUpdateChunk(chunk_size) => {
            if chunk_size > MAX_OTA_CHUNK_SIZE || chunk_size == 0 {
                return Ok(Response::InvalidRequest);
            }

            let Some(chunk) = client.next_update_chunk(chunk_size) else {
                error!(
                    "{}: Requested update when there is none available",
                    client.id()
                );

                return Ok(Response::InvalidRequest);
            };

            // if the entire blob has been sent
            if chunk.is_empty() {
                // mark update as finished/pushed
                db.send_os_update_stat(
                    client.id(),
                    client
                        .current_version()
                        .expect("Client's current version was not set"),
                    client
                        .update_version()
                        .expect("Client's update version was not set"),
                )
                .await?;

                // remove the blob from the client cache
                client.clear_update_blob();

                return Ok(Response::UpdateEnd);
            }

            Ok(Response::UpdatePart(chunk))
        }
        Request::ReportFirmwareUpdate(success) => {
            let Some(new_version) = client.update_version() else {
                error!(
                    "{}: Attempted to report a firmware update that was not recorded",
                    client.id()
                );
                return Ok(Response::InvalidRequest);
            };

            match db.mark_os_update_stat(client.id(), success).await {
                Err(Error::InvalidRequest) => Ok(Response::InvalidRequest),
                Err(why) => {
                    if notifs_cfg.on_update_failed {
                        notify_send(
                            notify,
                            client,
                            db,
                            format!("Failed to updated to {new_version}"),
                        )
                        .await?;
                    }

                    Err(why)
                }
                Ok(()) => {
                    if notifs_cfg.on_update_success {
                        notify_send(
                            notify,
                            client,
                            db,
                            format!("Successfully to updated to {new_version}"),
                        )
                        .await?;
                    }

                    Ok(Response::Ok)
                }
            }
        }
        Request::Bye => unreachable!(),
    }
}

async fn notify_send<S: AsRef<str>>(
    notify: &NotifySender,
    client: &Client<Authenticated>,
    db_client: &DatabaseClient,
    message: S,
) -> Result<(), Error> {
    // The push notification should include a node ID
    let push_message = format!("[Node #{}] {}", client.id(), message.as_ref()).into_boxed_str();

    // The database already links messages to nodes, so we don't need to include the ID.
    // Database has priority, so we store the notification there first.
    db_client
        .create_notification(client.id(), message.as_ref())
        .await?;

    // Push notifications come second, after we permanently stored the content in the database.

    // If the client has notifications muted, we won't send them
    if client.notifications_muted() {
        return Ok(());
    }

    // Since they are less important, we handle errors here and don't fail the whole request.
    if let Err(why) = notify.try_send(push_message) {
        warn!(
            "Node #{}: failed to queue push notification: {why}",
            client.id()
        );
    }

    Ok(())
}

use super::{
    client::{Authenticated, Client},
    config::Config,
    db::DatabaseClient,
    rate_limit::RateLimiter,
};
use crate::error::Error;
use pwmp_client::pwmp_msg::{request::Request, response::Response};
use std::{io::Read, net::SocketAddr, sync::Arc, time::Duration};
use tokio::{net::TcpStream, time::timeout};
use tracing::{debug, error, warn};

#[allow(clippy::needless_pass_by_value, clippy::cognitive_complexity)]
pub async fn handle_client(
    client: TcpStream,
    peer_addr: SocketAddr,
    db: &DatabaseClient,
    config: Arc<Config>,
) -> Result<(), Error> {
    let client = Client::new(client, peer_addr);
    let mut rate_limiter = RateLimiter::new(
        Duration::from_secs(config.rate_limits.time_frame),
        config.rate_limits.max_requests,
    );
    let mut client = client.authorize(db).await?;

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

        match handle_request(request, &mut client, db).await {
            Ok(response) => {
                if matches!(
                    response,
                    Response::InvalidRequest /* add more as needed */
                ) {
                    warn!(
                        "{}: Error while processing request: {response:?}",
                        client.id()
                    );
                }

                client.send_response(response).await?;
            }
            Err(why) => {
                client.shutdown(Some(Response::InternalServerError)).await?;
                return Err(why);
            }
        }
    }

    Ok(())
}

#[tracing::instrument(name = "handle_request()", skip_all, level = "debug", err, ret)]
async fn handle_request(
    req: Request,
    client: &mut Client<Authenticated>,
    db: &DatabaseClient,
) -> Result<Response, Error> {
    debug!("Handling {req:#?}");

    match req {
        Request::Ping => Ok(Response::Pong),
        Request::Handshake { .. } => {
            warn!("Received another handshake message");
            Ok(Response::InvalidRequest)
        }
        Request::PostResults {
            temperature,
            humidity,
            air_pressure,
        } => {
            if client.last_submit().is_some() {
                error!(
                    "{}: Submitted multiple posts, which is not allowed",
                    client.id()
                );
                return Ok(Response::InvalidRequest);
            }

            debug!(
                "{}: {temperature:.02}°C, {humidity}%, {air_pressure:?}hPa",
                client.id()
            );
            client.set_last_submit(
                db.post_results(client.id(), temperature, humidity, air_pressure)
                    .await?,
            );
            Ok(Response::Ok)
        }
        Request::PostStats {
            battery,
            wifi_ssid,
            wifi_rssi,
        } => {
            let Some(last_measurement_id) = client.last_submit() else {
                error!("{}: Missing measurement", client.id());
                return Ok(Response::InvalidRequest);
            };

            db.post_stats(last_measurement_id, battery, &wifi_ssid, wifi_rssi)
                .await?;
            Ok(Response::Ok)
        }
        Request::SendNotification(message) => {
            db.create_notification(client.id(), &message).await?;
            Ok(Response::Ok)
        }
        Request::GetSettings => {
            let values = db.get_settings(client.id()).await?;

            if values.is_none() {
                warn!("{}: Settings are undefined", client.id());
            }

            Ok(Response::Settings(values))
        }
        Request::UpdateCheck(current_ver) => {
            debug!("{}: Claims OS version {}", client.id(), current_ver);
            let update_info = db.check_os_update(client.id(), current_ver).await?;

            if let Some((version, firmware_blob)) = update_info {
                client.store_update_check_result(current_ver, version, firmware_blob);
                Ok(Response::UpdateAvailable(version))
            } else {
                client.mark_up_to_date();
                Ok(Response::FirmwareUpToDate)
            }
        }
        Request::NextUpdateChunk(chunk_size) => {
            let Some(reader) = client.update_chunk() else {
                error!(
                    "{}: Requested update when there is none available",
                    client.id()
                );

                return Ok(Response::InvalidRequest);
            };

            let mut buf = vec![0; chunk_size];
            let read = reader.read(&mut buf)?;
            let chunk = &buf[..read];

            if chunk.is_empty() {
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
                return Ok(Response::UpdateEnd);
            }

            Ok(Response::UpdatePart(buf.into_boxed_slice()))
        }
        Request::ReportFirmwareUpdate(success) => {
            db.mark_os_update_stat(client.id(), success).await?;
            Ok(Response::Ok)
        }
        Request::Bye => unreachable!(),
    }
}

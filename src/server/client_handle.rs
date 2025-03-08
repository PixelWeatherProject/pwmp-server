use super::{
    client::{Authenticated, Client},
    config::Config,
    db::DatabaseClient,
    rate_limit::RateLimiter,
};
use crate::error::Error;
use log::{debug, error, warn};
use pwmp_client::pwmp_msg::{Message, request::Request, response::Response};
use std::{
    io::{self, Read},
    net::TcpStream,
    sync::Arc,
    time::Duration,
};

#[allow(clippy::needless_pass_by_value)]
pub fn handle_client(
    client: TcpStream,
    db: &DatabaseClient,
    config: Arc<Config>,
) -> Result<(), Error> {
    let client = Client::new(client)?;
    let mut rate_limiter = RateLimiter::new(
        Duration::from_secs(config.rate_limits.time_frame),
        config.rate_limits.max_requests,
    );
    let mut client = client.authorize(db)?;

    loop {
        if client.time_since_last_interaction() >= config.max_stall_time / 2 {
            warn!("{}: Is stalling", client.id());
        }

        if client.time_since_last_interaction() >= config.max_stall_time {
            error!("{}: Stalled for too long, kicking", client.id());
            return Err(Error::StallTimeExceeded);
        }

        let request = match client.await_request() {
            Ok(req) => req,
            Err(Error::Io(err)) if err.kind() == io::ErrorKind::TimedOut => {
                continue;
            }
            Err(other) => return Err(other),
        };

        if rate_limiter.hit() {
            error!("{}: Exceeded request limits", client.id());
            break;
        }

        if request == Request::Bye {
            debug!("{}: Bye", client.id());
            client.shutdown()?;
            break;
        }

        let response = handle_request(request, &mut client, db)?.ok_or(Error::BadRequest)?;

        client.send_response(response)?;
    }

    Ok(())
}

fn handle_request(
    req: Request,
    client: &mut Client<Authenticated>,
    db: &DatabaseClient,
) -> Result<Option<Response>, Error> {
    debug!(
        "Handling {req:#?} ({} bytes)",
        Message::Request(req.clone()).size()
    );

    match req {
        Request::Ping => Ok(Some(Response::Pong)),
        Request::Hello { .. } => {
            warn!("Received double `Hello` messages");
            Ok(None)
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
                return Ok(None);
            }

            debug!(
                "{}: {temperature}C, {humidity}%, {air_pressure:?}hPa",
                client.id()
            );
            client.set_last_submit(db.post_results(
                client.id(),
                temperature,
                humidity,
                air_pressure,
            )?);
            Ok(Some(Response::Ok))
        }
        Request::PostStats {
            ref battery,
            wifi_ssid,
            wifi_rssi,
        } => {
            let Some(last_measurement_id) = client.last_submit() else {
                error!("{}: Missing measurement", client.id());
                return Ok(None);
            };

            db.post_stats(last_measurement_id, battery, &wifi_ssid, wifi_rssi)?;
            Ok(Some(Response::Ok))
        }
        Request::SendNotification(message) => {
            db.create_notification(client.id(), &message)?;
            Ok(Some(Response::Ok))
        }
        Request::GetSettings => {
            let values = db.get_settings(client.id())?;

            if values.is_none() {
                warn!("{}: Settings are undefined", client.id());
            }

            Ok(Some(Response::Settings(values)))
        }
        Request::UpdateCheck(current_ver) => {
            debug!("{}: Claims OS version {}", client.id(), current_ver);
            let update_info = db.check_os_update(client.id(), current_ver)?;

            if let Some((version, firmware_blob)) = update_info {
                client.store_update_check_result(current_ver, version, firmware_blob);
                Ok(Some(Response::UpdateAvailable(version)))
            } else {
                client.mark_up_to_date();
                Ok(Some(Response::FirmwareUpToDate))
            }
        }
        Request::NextUpdateChunk(chunk_size) => {
            let Some(reader) = client.update_chunk() else {
                error!(
                    "{}: Requested update when there is none available",
                    client.id()
                );

                return Ok(None);
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
                )?;
                return Ok(Some(Response::UpdateEnd));
            }

            Ok(Some(Response::UpdatePart(buf.into_boxed_slice())))
        }
        Request::ReportFirmwareUpdate(success) => {
            db.mark_os_update_stat(client.id(), success)?;
            Ok(Some(Response::Ok))
        }
        Request::Bye => unreachable!(),
    }
}

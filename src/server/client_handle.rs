use super::{
    client::{Authenticated, Client},
    config::Config,
    db::DatabaseClient,
    rate_limit::RateLimiter,
};
use crate::error::Error;
use log::{debug, error, warn};
use pwmp_client::pwmp_msg::{request::Request, response::Response};
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
    let client = Client::new(client);
    let mut rate_limiter = RateLimiter::new(
        Duration::from_secs(config.rate_limits.time_frame),
        config.rate_limits.max_requests,
    );
    let mut client = client.authorize(db)?;

    loop {
        let request = match client.receive_request() {
            Ok(req) => req,
            Err(Error::Io(err)) if err.kind() == io::ErrorKind::WouldBlock => {
                error!("{}: Stalled for too long, kicking", client.id());
                let _ = client.shutdown(Some(Response::Stalling));
                return Err(Error::StallTimeExceeded);
            }
            Err(other) => return Err(other),
        };

        if rate_limiter.hit() {
            error!("{}: Exceeded request limits", client.id());
            client.shutdown(Some(Response::RateLimitExceeded))?;
            break;
        }

        if request == Request::Bye {
            debug!("{}: Bye", client.id());
            client.shutdown(None)?;
            break;
        }

        match handle_request(request, &mut client, db) {
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

                client.send_response(response)?;
            }
            Err(why) => {
                client.shutdown(Some(Response::InternalServerError))?;
                return Err(why);
            }
        }
    }

    Ok(())
}

fn handle_request(
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
                "{}: {temperature}C, {humidity}%, {air_pressure:?}hPa",
                client.id()
            );
            client.set_last_submit(db.post_results(
                client.id(),
                temperature,
                humidity,
                air_pressure,
            )?);
            Ok(Response::Ok)
        }
        Request::PostStats {
            ref battery,
            wifi_ssid,
            wifi_rssi,
        } => {
            let Some(last_measurement_id) = client.last_submit() else {
                error!("{}: Missing measurement", client.id());
                return Ok(Response::InvalidRequest);
            };

            db.post_stats(last_measurement_id, battery, &wifi_ssid, wifi_rssi)?;
            Ok(Response::Ok)
        }
        Request::SendNotification(message) => {
            db.create_notification(client.id(), &message)?;
            Ok(Response::Ok)
        }
        Request::GetSettings => {
            let values = db.get_settings(client.id())?;

            if values.is_none() {
                warn!("{}: Settings are undefined", client.id());
            }

            Ok(Response::Settings(values))
        }
        Request::UpdateCheck(current_ver) => {
            debug!("{}: Claims OS version {}", client.id(), current_ver);
            let update_info = db.check_os_update(client.id(), current_ver)?;

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
                )?;
                return Ok(Response::UpdateEnd);
            }

            Ok(Response::UpdatePart(buf.into_boxed_slice()))
        }
        Request::ReportFirmwareUpdate(success) => {
            db.mark_os_update_stat(client.id(), success)?;
            Ok(Response::Ok)
        }
        Request::Bye => unreachable!(),
    }
}

use super::{
    client::{Authenticated, Client},
    db::DatabaseClient,
};
use crate::error::Error;
use log::{debug, error, warn};
use pwmp_client::pwmp_msg::{Message, request::Request, response::Response};
use std::io::Read;

pub fn handle_request(
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
            warn!("Received duplicate `Hello` messages");
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

use pwmp_client::{
    PwmpClient,
    ota::UpdateStatus,
    pwmp_msg::{MsgId, mac::Mac, version::Version},
};
use std::{
    process::exit,
    str::FromStr,
    sync::atomic::{AtomicU32, Ordering},
    time::{Duration, Instant},
};
use tracing::{debug, error, info};

static COUNTER: AtomicU32 = AtomicU32::new(1);

/// Try to connect to a server and authenticate with the given MAC address to
/// check if the server is working properly.
#[allow(
    clippy::needless_pass_by_value,
    clippy::cognitive_complexity,
    clippy::too_many_lines
)]
pub fn test(host: String, port: Option<u16>, raw_mac: String) {
    let Ok(mac) = Mac::from_str(&raw_mac) else {
        error!("Invalid MAC address format");
        return;
    };

    let mut response_times = Vec::new();

    let full_addr = format!("{}:{}", host, port.unwrap_or(55300));
    let start = Instant::now();

    let mut client = match qbench(
        "connect",
        || PwmpClient::new(full_addr, &id_generator, None, None, None),
        &mut response_times,
    ) {
        Ok(client) => {
            info!("Client connected successfully!");
            client
        }
        Err(why) => {
            error!("Failed to test connection: {why}");
            exit(1);
        }
    };

    info!("Performing handshake");
    if let Err(why) = qbench(
        "handshake",
        || client.perform_handshake(mac),
        &mut response_times,
    ) {
        error!("Handshake failed: {why}");
        exit(1);
    }

    debug!("Pinging");
    if !qbench("ping", || client.ping(), &mut response_times) {
        error!("Ping test failed");
        exit(1);
    }

    debug!("Requesting settings");
    if let Err(why) = qbench(
        "get settings",
        || client.get_settings(),
        &mut response_times,
    ) {
        error!("Failed to get settings: {why}");
    }

    debug!("Testing measurement posting");
    if let Err(why) = qbench(
        "post 1",
        || client.post_measurements(0.00, 0, Some(0)),
        &mut response_times,
    ) {
        error!("Failed: {why}");
        exit(1);
    }

    debug!("Testing stats posting");
    if let Err(why) = qbench(
        "post 2",
        || client.post_stats(3.70, "<PWMP Test>", -50),
        &mut response_times,
    ) {
        error!("Failed: {why}");
        exit(1);
    }

    debug!("Testing OTA API");
    match qbench(
        "ota",
        || client.check_os_update(Version::new(0, 0, 0)),
        &mut response_times,
    ) {
        Ok(UpdateStatus::Available(..)) => {
            loop {
                debug!("Testing update chunk request");

                match client.next_update_chunk(None) {
                    Err(why) => {
                        error!("Failed: {why}");
                        exit(1);
                    }
                    Ok(None) => break,
                    Ok(..) => (),
                }
            }

            debug!("Testing firmware report");
            if let Err(why) = client.report_firmware(false) {
                error!("Failed: {why}");
                exit(1);
            }
        }
        Ok(_) => (),
        Err(why) => {
            error!("Failed: {why}");
            exit(1);
        }
    }

    debug!("Testing notification posting");
    if let Err(why) = qbench(
        "notification",
        || client.send_notification("Example notification"),
        &mut response_times,
    ) {
        error!("Failed: {why}");
        exit(1);
    }

    let elapsed = start.elapsed();
    info!("Test passed in {elapsed:?}!");

    let avg_response_time =
        Duration::from_micros(response_times.iter().sum::<u64>() / response_times.len() as u64);
    let min_response_time = Duration::from_micros(*response_times.iter().min().unwrap());
    let max_response_time = Duration::from_micros(*response_times.iter().max().unwrap());
    info!(
        "Response timing: min={min_response_time:?}, max={max_response_time:?}, avg={avg_response_time:?}"
    );
}

fn id_generator() -> MsgId {
    COUNTER.fetch_add(1, Ordering::SeqCst)
}

fn qbench<F: FnOnce() -> U, U>(op: &'static str, what: F, response_time_store: &mut Vec<u64>) -> U {
    let start = Instant::now();

    let res = what();

    let end = start.elapsed();
    debug!("{op} took {end:?}");
    response_time_store.push(end.as_micros().try_into().unwrap());

    res
}

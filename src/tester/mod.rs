use log::{debug, error, info};
use pwmp_client::{
    PwmpClient,
    ota::UpdateStatus,
    pwmp_msg::mac::Mac,
    pwmp_msg::{Decimal, dec, version::Version},
};
use std::{process::exit, str::FromStr};

/// Try to connect to a server and authenticate with the given MAC address to
/// check if the server is working properly.
#[allow(clippy::needless_pass_by_value, clippy::cognitive_complexity)]
pub fn test(host: String, port: Option<u16>, raw_mac: String) {
    let Ok(mac) = Mac::from_str(&raw_mac) else {
        error!("Invalid MAC address format");
        return;
    };

    let full_addr = format!("{}:{}", host, port.unwrap_or(55300));

    let mut client = match PwmpClient::new(full_addr, &crate::csprng, None, None, None) {
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
    if let Err(why) = client.perform_handshake(mac) {
        error!("Handshake failed: {why}");
    }

    debug!("Pinging");
    if !client.ping() {
        error!("Ping test failed");
        exit(1);
    }

    debug!("Requesting settings");
    if let Err(why) = client.get_settings() {
        error!("Failed to get settings: {why}");
    }

    debug!("Testing measurement posting");
    if let Err(why) = client.post_measurements(dec!(0.00), 0, Some(0)) {
        error!("Failed: {why}");
        exit(1);
    }

    debug!("Testing stats posting");
    if let Err(why) = client.post_stats(dec!(3.70), "<PWMP Test>", -50) {
        error!("Failed: {why}");
        exit(1);
    }

    debug!("Testing OTA API");
    match client.check_os_update(Version::new(0, 0, 0)) {
        Ok(UpdateStatus::Available(..)) => {
            debug!("Testing update chunk request");
            if let Err(why) = client.next_update_chunk(None) {
                error!("Failed: {why}");
                exit(1);
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
    if let Err(why) = client.send_notification("Example notification") {
        error!("Failed: {why}");
        exit(1);
    }

    info!("Test passed!");
}

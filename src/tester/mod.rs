use log::{error, info};
use pwmp_client::{pwmp_msg::mac::Mac, PwmpClient};
use std::str::FromStr;

/// Try to connect to a server and authenticate with the given MAC address to
/// check if the server is working properly.
#[allow(clippy::needless_pass_by_value)]
pub fn test(host: String, port: Option<u16>, raw_mac: String) {
    let Ok(mac) = Mac::from_str(&raw_mac) else {
        error!("Invalid MAC address format");
        return;
    };

    let full_addr = format!("{}:{}", host, port.unwrap_or(55300));

    match PwmpClient::new(full_addr, mac, None, None, None) {
        Ok(_) => info!("Client connected successfully!"),
        Err(why) => error!("Failed to test connection: {why}"),
    };
}

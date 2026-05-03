mod hassnotify;
mod pushsafer;

use crate::{error::Error, server::config::NotificationServiceConfig};
use hassnotify::HassNotifyClient;
use pushsafer::PushsaferClient;

#[derive(Debug)]
pub enum NotificationClient {
    None,
    HassNotify(HassNotifyClient),
    Pushsafer(PushsaferClient),
}

impl NotificationClient {
    pub fn new(config: Option<&NotificationServiceConfig>) -> Result<Self, Error> {
        match config {
            None => Ok(Self::None),
            Some(service) => match service {
                NotificationServiceConfig::Pushsafer { api_key, device } => {
                    let client = PushsaferClient::new(device, api_key)?;
                    Ok(Self::Pushsafer(client))
                }
                NotificationServiceConfig::HassNotify { url, token, target } => {
                    let client = HassNotifyClient::new(target, token, url)?;
                    Ok(Self::HassNotify(client))
                }
            },
        }
    }

    pub async fn send_notification(&mut self, content: &str) -> Result<(), Error> {
        match self {
            Self::HassNotify(client) => client.send_notification(content).await?,
            Self::Pushsafer(client) => client.send_notification(content).await?,
            Self::None => (),
        }

        Ok(())
    }
}

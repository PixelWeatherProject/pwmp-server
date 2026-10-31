use crate::error::Error;
use reqwest::{Client, Url, header::HeaderValue};
use serde::Serialize;
use std::time::Duration;

#[derive(Debug)]
pub struct HassNotifyClient {
    client: Client,
    url: Url,
    token: HeaderValue,
}

#[derive(Serialize)]
struct RequestBody<'l> {
    title: &'l str,
    content: &'l str,
}

impl HassNotifyClient {
    pub fn new(device: &str, token: &str, url: &str) -> Result<Self, Error> {
        let client = reqwest::ClientBuilder::new()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(5))
            .build()?;

        let mut url = Url::parse(url)?;
        let resource = format!("api/services/notify/{device}");
        url.set_path(&resource);

        let mut token =
            HeaderValue::from_str(&format!("Bearer {token}")).expect("Invalid header value");
        token.set_sensitive(true);

        Ok(Self { client, url, token })
    }

    #[tracing::instrument(
        name = "HassNotifyClient::send_notification()",
        level = "debug",
        skip(self),
        err,
        ret
    )]
    pub async fn send_notification(&self, content: &str) -> Result<(), Error> {
        self.client
            .post(self.url.clone())
            .header("Authorization", self.token.clone())
            .json(&RequestBody {
                title: "PixelWeather",
                content,
            })
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }
}

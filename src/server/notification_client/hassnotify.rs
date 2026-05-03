use crate::error::Error;
use reqwest::{Client, Url};
use serde_json::json;

#[derive(Debug)]
pub struct HassNotifyClient {
    client: Client,
    url: Url,
    token: Box<str>,
}

impl HassNotifyClient {
    pub fn new(device: &str, token: &str, url: &str) -> Result<Self, Error> {
        let client = reqwest::Client::new();

        let mut url = Url::parse(url)?;
        let resource = format!("api/services/notify/{device}");
        url.set_path(&resource);

        Ok(Self {
            client,
            url,
            token: token.into(),
        })
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
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Content-Type", "application/json")
            .json(&json!({
                "title": "PixelWeather",
                "message": content,
            }))
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }
}

use crate::error::Error;
use reqwest::{Client, Url};

#[derive(Debug)]
pub struct PushsaferClient {
    client: Client,
    url: Url,
}

impl PushsaferClient {
    pub fn new(device: &str, api_key: &str) -> Result<Self, Error> {
        let client = reqwest::Client::new();
        let url = Url::parse_with_params(
            "https://www.pushsafer.com/api",
            &[
                ("k", api_key),
                ("d", device),
                ("t", "PixelWeather"),
                ("i", "2"),
                ("pr", "1"),
            ],
        )?;

        Ok(Self { client, url })
    }

    pub async fn send_notification(&mut self, content: &str) -> Result<(), Error> {
        let final_url = self
            .url
            .query_pairs_mut()
            .append_pair("m", content)
            .finish()
            .to_string();

        self.client
            .post(final_url)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }
}

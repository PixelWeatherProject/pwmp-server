use crate::{
    error::Error,
    server::config::{Config, DatabaseConfig},
};
use async_trait::async_trait;
use pwmp_client::pwmp_msg::{
    aliases::{AirPressure, BatteryVoltage, Humidity, Rssi, Temperature},
    mac::Mac,
    settings::NodeSettings,
    version::Version,
};

mod postgres;
mod sqlite;

pub type NodeId = i32;
pub type MeasurementId = i32;
pub type FirmwareBlob = Box<[u8]>;
pub type UpdateStatId = i32;

pub struct DatabaseClient(Box<dyn Backend>);

#[async_trait]
pub trait Backend: Send + Sync {
    async fn setup_timezone(&self, tz: &str) -> Result<(), Error>;

    async fn authorize_device(&self, mac: &Mac) -> Result<Option<NodeId>, Error>;

    async fn create_notification(&self, node_id: NodeId, content: &str) -> Result<(), Error>;

    async fn get_settings(&self, node_id: NodeId) -> Result<Option<NodeSettings>, Error>;

    async fn post_results(
        &self,
        node: NodeId,
        temp: Temperature,
        hum: Humidity,
        air_p: Option<AirPressure>,
    ) -> Result<MeasurementId, Error>;

    async fn post_stats(
        &self,
        measurement: MeasurementId,
        battery: BatteryVoltage,
        wifi_ssid: &str,
        wifi_rssi: Rssi,
    ) -> Result<(), Error>;

    async fn run_migrations(&self) -> Result<(), Error>;

    async fn check_os_update(
        &self,
        node: NodeId,
        current_ver: Version,
    ) -> Result<Option<(Version, FirmwareBlob)>, Error>;

    async fn send_os_update_stat(
        &self,
        node_id: NodeId,
        old_ver: Version,
        new_ver: Version,
    ) -> Result<UpdateStatId, Error>;

    async fn mark_os_update_stat(&self, node_id: NodeId, success: bool) -> Result<(), Error>;

    async fn erase(&self, content_only: bool, keep_devices: bool) -> Result<(), Error>;
}

impl DatabaseClient {
    pub async fn new(config: &Config) -> Result<Self, Error> {
        let backend: Box<dyn Backend> = match &config.database {
            DatabaseConfig::Postgres(config) => {
                Box::new(postgres::PostgresClient::new(config).await?)
            }
            DatabaseConfig::Sqlite(config) => Box::new(sqlite::SqliteClient::new(config).await?),
        };

        Ok(Self(backend))
    }
}

#[async_trait]
impl Backend for DatabaseClient {
    async fn setup_timezone(&self, tz: &str) -> Result<(), Error> {
        self.setup_timezone(tz).await
    }

    async fn authorize_device(&self, mac: &Mac) -> Result<Option<NodeId>, Error> {
        self.authorize_device(mac).await
    }

    async fn create_notification(&self, node_id: NodeId, content: &str) -> Result<(), Error> {
        self.create_notification(node_id, content).await
    }

    async fn get_settings(&self, node_id: NodeId) -> Result<Option<NodeSettings>, Error> {
        self.get_settings(node_id).await
    }

    async fn post_results(
        &self,
        node: NodeId,
        temp: Temperature,
        hum: Humidity,
        air_p: Option<AirPressure>,
    ) -> Result<MeasurementId, Error> {
        self.post_results(node, temp, hum, air_p).await
    }

    async fn post_stats(
        &self,
        measurement: MeasurementId,
        battery: BatteryVoltage,
        wifi_ssid: &str,
        wifi_rssi: Rssi,
    ) -> Result<(), Error> {
        self.post_stats(measurement, battery, wifi_ssid, wifi_rssi)
            .await
    }

    async fn run_migrations(&self) -> Result<(), Error> {
        self.run_migrations().await
    }

    async fn check_os_update(
        &self,
        node: NodeId,
        current_ver: Version,
    ) -> Result<Option<(Version, FirmwareBlob)>, Error> {
        self.check_os_update(node, current_ver).await
    }

    async fn send_os_update_stat(
        &self,
        node_id: NodeId,
        old_ver: Version,
        new_ver: Version,
    ) -> Result<UpdateStatId, Error> {
        self.send_os_update_stat(node_id, old_ver, new_ver).await
    }

    async fn mark_os_update_stat(&self, node_id: NodeId, success: bool) -> Result<(), Error> {
        self.mark_os_update_stat(node_id, success).await
    }

    async fn erase(&self, content_only: bool, keep_devices: bool) -> Result<(), Error> {
        self.erase(content_only, keep_devices).await
    }
}

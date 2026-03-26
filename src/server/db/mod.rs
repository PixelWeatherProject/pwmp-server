use crate::{
    error::Error,
    server::config::{Config, DatabaseConfig},
};
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
pub type SleepTime = i16;

pub enum DatabaseClient {
    Postgres(postgres::PostgresClient),
    Sqlite(sqlite::SqliteClient),
}

#[derive(Debug, Clone, Copy)]
pub enum EraseOptions {
    Everything,
    ContentOnly { keep_devices: bool },
}

#[derive(Debug, Clone)]
pub struct FirmwareEntry {
    pub id: i32,
    pub version: Version,
    pub size: i32,
    pub blob: Vec<u8>,
    pub added: String,
    pub restrict: Option<Vec<NodeId>>,
}

pub trait DatabaseBackend {
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

    async fn erase(&self, options: EraseOptions) -> Result<(), Error>;

    async fn get_firmwares(&self) -> Result<Vec<FirmwareEntry>, Error>;
}

impl DatabaseClient {
    #[tracing::instrument(name = "DatabaseClient::new()", level = "debug", err, skip_all)]
    pub async fn new(config: &Config) -> Result<Self, Error> {
        match &config.database {
            DatabaseConfig::Postgres {
                host,
                port,
                user,
                password,
                name,
                ssl,
            } => Ok(Self::Postgres(
                postgres::PostgresClient::new(host, *port, user, password, name, *ssl).await?,
            )),
            DatabaseConfig::Sqlite { file } => {
                Ok(Self::Sqlite(sqlite::SqliteClient::new(file).await?))
            }
        }
    }
}

impl DatabaseBackend for DatabaseClient {
    async fn authorize_device(&self, mac: &Mac) -> Result<Option<NodeId>, Error> {
        match self {
            Self::Postgres(client) => client.authorize_device(mac).await,
            Self::Sqlite(client) => client.authorize_device(mac).await,
        }
    }

    async fn create_notification(&self, node_id: NodeId, content: &str) -> Result<(), Error> {
        match self {
            Self::Postgres(client) => client.create_notification(node_id, content).await,
            Self::Sqlite(client) => client.create_notification(node_id, content).await,
        }
    }

    async fn get_settings(&self, node_id: NodeId) -> Result<Option<NodeSettings>, Error> {
        match self {
            Self::Postgres(client) => client.get_settings(node_id).await,
            Self::Sqlite(client) => client.get_settings(node_id).await,
        }
    }

    async fn post_results(
        &self,
        node: NodeId,
        temp: Temperature,
        hum: Humidity,
        air_p: Option<AirPressure>,
    ) -> Result<MeasurementId, Error> {
        match self {
            Self::Postgres(client) => client.post_results(node, temp, hum, air_p).await,
            Self::Sqlite(client) => client.post_results(node, temp, hum, air_p).await,
        }
    }

    async fn post_stats(
        &self,
        measurement: MeasurementId,
        battery: BatteryVoltage,
        wifi_ssid: &str,
        wifi_rssi: Rssi,
    ) -> Result<(), Error> {
        match self {
            Self::Postgres(client) => {
                client
                    .post_stats(measurement, battery, wifi_ssid, wifi_rssi)
                    .await
            }
            Self::Sqlite(client) => {
                client
                    .post_stats(measurement, battery, wifi_ssid, wifi_rssi)
                    .await
            }
        }
    }

    async fn run_migrations(&self) -> Result<(), Error> {
        match self {
            Self::Postgres(client) => client.run_migrations().await,
            Self::Sqlite(client) => client.run_migrations().await,
        }
    }

    async fn check_os_update(
        &self,
        node: NodeId,
        current_ver: Version,
    ) -> Result<Option<(Version, FirmwareBlob)>, Error> {
        match self {
            Self::Postgres(client) => client.check_os_update(node, current_ver).await,
            Self::Sqlite(client) => client.check_os_update(node, current_ver).await,
        }
    }

    async fn send_os_update_stat(
        &self,
        node_id: NodeId,
        old_ver: Version,
        new_ver: Version,
    ) -> Result<UpdateStatId, Error> {
        match self {
            Self::Postgres(client) => client.send_os_update_stat(node_id, old_ver, new_ver).await,
            Self::Sqlite(client) => client.send_os_update_stat(node_id, old_ver, new_ver).await,
        }
    }

    async fn mark_os_update_stat(&self, node_id: NodeId, success: bool) -> Result<(), Error> {
        match self {
            Self::Postgres(client) => client.mark_os_update_stat(node_id, success).await,
            Self::Sqlite(client) => client.mark_os_update_stat(node_id, success).await,
        }
    }

    async fn erase(&self, options: EraseOptions) -> Result<(), Error> {
        match self {
            Self::Postgres(client) => client.erase(options).await,
            Self::Sqlite(client) => client.erase(options).await,
        }
    }

    async fn get_firmwares(&self) -> Result<Vec<FirmwareEntry>, Error> {
        match self {
            Self::Postgres(client) => client.get_firmwares().await,
            Self::Sqlite(client) => client.get_firmwares().await,
        }
    }
}

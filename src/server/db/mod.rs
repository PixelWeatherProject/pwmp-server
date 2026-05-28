use crate::{
    error::Error,
    server::{
        config::{Config, DatabaseConfig},
        db::{postgres::PostgresClient, sqlite::SqliteClient},
    },
};
use moka::future::Cache;
use pwmp_client::pwmp_msg::{
    aliases::{AirPressure, BatteryVoltage, Humidity, Rssi, Temperature},
    mac::Mac,
    settings::NodeSettings,
    version::Version,
};
use tracing::debug;

mod postgres;
mod sqlite;

pub type NodeId = i32;
pub type MeasurementId = i32;
pub type FirmwareBlob = Box<[u8]>;
pub type UpdateStatId = i32;
pub type SleepTime = i16;

type NodeIdCache = Cache<Mac, Option<NodeId>>;
type NodeSettingsCache = Cache<NodeId, Option<NodeSettings>>;

pub struct DatabaseClient {
    backend: Box<dyn DatabaseBackend>,
    node_id_cache: NodeIdCache,
    node_settings_cache: NodeSettingsCache,
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

#[async_trait::async_trait]
pub trait DatabaseBackend: Send + Sync {
    async fn authorize_device(&self, mac: &Mac) -> Result<Option<NodeId>, Error>;

    async fn create_notification(&self, node_id: NodeId, content: &str) -> Result<(), Error>;

    async fn get_settings(&self, node_id: NodeId) -> Result<Option<NodeSettings>, Error>;

    #[allow(clippy::too_many_arguments)]
    async fn post_measurements(
        &self,
        node: NodeId,
        temp: Temperature,
        hum: Humidity,
        air_p: Option<AirPressure>,
        cpu_temp: Temperature,
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

    async fn upload_firmware(
        &self,
        blob: Vec<u8>,
        version: Version,
        restrict_nodes: Option<Vec<NodeId>>,
    ) -> Result<(), Error>;
}

impl DatabaseClient {
    #[tracing::instrument(name = "DatabaseClient::new()", level = "debug", err, skip_all)]
    pub async fn new(config: &Config) -> Result<Self, Error> {
        let node_id_cache = NodeIdCache::builder()
            .max_capacity(config.cache.auth_capacity)
            .time_to_live(config.cache.auth_ttl)
            .async_eviction_listener(|k, v, c| {
                Box::pin(async move {
                    debug!("Auth cache evicted mapping '{k}'<=>'{v:?}': {c:?}");
                })
            })
            .build();
        let node_settings_cache = NodeSettingsCache::builder()
            .max_capacity(config.cache.settings_capacity)
            .time_to_live(config.cache.settings_ttl)
            .async_eviction_listener(|k, _, c| {
                Box::pin(async move {
                    debug!("Settings cache evicted node '{k}': {c:?}");
                })
            })
            .build();

        match &config.database {
            DatabaseConfig::Postgres {
                host,
                port,
                user,
                password,
                name,
                ssl,
            } => Ok(Self {
                backend: Box::new(
                    PostgresClient::new(host, *port, user, password, name, *ssl).await?,
                ),
                node_id_cache,
                node_settings_cache,
            }),
            DatabaseConfig::Sqlite { file } => Ok(Self {
                backend: Box::new(SqliteClient::new(file).await?),
                node_id_cache,
                node_settings_cache,
            }),
        }
    }
}

#[async_trait::async_trait]
impl DatabaseBackend for DatabaseClient {
    async fn authorize_device(&self, mac: &Mac) -> Result<Option<NodeId>, Error> {
        if let Some(maybe_id) = self.node_id_cache.get(mac).await {
            debug!("Auth cache hit for '{mac}' -> '{maybe_id:?}'");
            return Ok(maybe_id);
        }

        debug!("Auth cache miss for '{mac}'");

        let maybe_id = self.backend.authorize_device(mac).await?;
        self.node_id_cache.insert(*mac, maybe_id).await;
        Ok(None)
    }

    async fn create_notification(&self, node_id: NodeId, content: &str) -> Result<(), Error> {
        self.backend.create_notification(node_id, content).await
    }

    async fn get_settings(&self, node_id: NodeId) -> Result<Option<NodeSettings>, Error> {
        if let Some(settings) = self.node_settings_cache.get(&node_id).await {
            debug!("Settings cache hit for '{node_id}'");
            return Ok(settings);
        }

        debug!("Settings cache miss for '{node_id}'");

        let settings = self.backend.get_settings(node_id).await?;
        self.node_settings_cache.insert(node_id, settings).await;
        Ok(None)
    }

    async fn post_measurements(
        &self,
        node: NodeId,
        temp: Temperature,
        hum: Humidity,
        air_p: Option<AirPressure>,
        cpu_temp: Temperature,
        battery: BatteryVoltage,
        wifi_ssid: &str,
        wifi_rssi: i8,
    ) -> Result<(), Error> {
        self.backend
            .post_measurements(
                node, temp, hum, air_p, cpu_temp, battery, wifi_ssid, wifi_rssi,
            )
            .await
    }

    async fn run_migrations(&self) -> Result<(), Error> {
        self.backend.run_migrations().await
    }

    async fn check_os_update(
        &self,
        node: NodeId,
        current_ver: Version,
    ) -> Result<Option<(Version, FirmwareBlob)>, Error> {
        self.backend.check_os_update(node, current_ver).await
    }

    async fn send_os_update_stat(
        &self,
        node_id: NodeId,
        old_ver: Version,
        new_ver: Version,
    ) -> Result<UpdateStatId, Error> {
        self.backend
            .send_os_update_stat(node_id, old_ver, new_ver)
            .await
    }

    async fn mark_os_update_stat(&self, node_id: NodeId, success: bool) -> Result<(), Error> {
        self.backend.mark_os_update_stat(node_id, success).await
    }

    async fn erase(&self, options: EraseOptions) -> Result<(), Error> {
        self.backend.erase(options).await
    }

    async fn get_firmwares(&self) -> Result<Vec<FirmwareEntry>, Error> {
        self.backend.get_firmwares().await
    }

    async fn upload_firmware(
        &self,
        blob: Vec<u8>,
        version: Version,
        restrict_nodes: Option<Vec<NodeId>>,
    ) -> Result<(), Error> {
        self.backend
            .upload_firmware(blob, version, restrict_nodes)
            .await
    }
}

impl EraseOptions {
    pub const fn new(content_only: bool, keep_devices: bool) -> Self {
        if content_only {
            Self::ContentOnly { keep_devices }
        } else {
            Self::Everything
        }
    }
}

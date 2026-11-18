use crate::{
    error::Error,
    server::{
        config::{Config, DatabaseConfig},
        db::{postgres::PostgresClient, sqlite::SqliteClient},
    },
};
use moka::future::Cache;
use pwmp_client::pwmp_msg::{
    aliases::Temperature,
    mac::Mac,
    measurement::{HistoricMeasurement, MeasurementCollection},
    settings::NodeSettings,
    version::Version,
    wlan_stats::WiFiStats,
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
    backend: Box<DynDatabaseBackend<'static>>,
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

/// Contract implemented by each concrete database backend.
///
/// `dynosaur` generates `DynDatabaseBackend`, a boxed, dyn-compatible wrapper
/// used by [`DatabaseClient`]. Implementors write plain `async fn`s.
#[dynosaur::dynosaur(DynDatabaseBackend = dyn(box) DatabaseBackend)]
trait DatabaseBackend: Send + Sync {
    fn authorize_device(
        &self,
        mac: &Mac,
    ) -> impl Future<Output = Result<Option<NodeId>, Error>> + Send;

    fn create_notification(
        &self,
        node_id: NodeId,
        content: &str,
    ) -> impl Future<Output = Result<(), Error>> + Send;

    fn get_settings(
        &self,
        node_id: NodeId,
    ) -> impl Future<Output = Result<Option<NodeSettings>, Error>> + Send;

    fn post_measurements(
        &self,
        node: NodeId,
        measurements: MeasurementCollection,
        wifi_stats: WiFiStats,
        os_version: Version,
        os_version_ext: Option<&str>,
    ) -> impl Future<Output = Result<(), Error>> + Send;

    fn post_historic_measurements(
        &self,
        node: NodeId,
        measurements: Box<[HistoricMeasurement]>,
        wifi_stats: &WiFiStats,
        os_version: Version,
        os_version_ext: Option<&str>,
    ) -> impl Future<Output = Result<(), Error>> + Send;

    fn store_diagnostic_artifact(
        &self,
        node: NodeId,
        tag: &str,
        content: &str,
    ) -> impl Future<Output = Result<(), Error>> + Send;

    fn run_migrations(&self) -> impl Future<Output = Result<(), Error>> + Send;

    fn check_os_update(
        &self,
        node: NodeId,
        current_ver: Version,
    ) -> impl Future<Output = Result<Option<(Version, FirmwareBlob)>, Error>> + Send;

    fn send_os_update_stat(
        &self,
        node_id: NodeId,
        old_ver: Version,
        new_ver: Version,
    ) -> impl Future<Output = Result<UpdateStatId, Error>> + Send;

    fn mark_os_update_stat(
        &self,
        node_id: NodeId,
        success: bool,
    ) -> impl Future<Output = Result<(), Error>> + Send;

    fn erase(&self, options: EraseOptions) -> impl Future<Output = Result<(), Error>> + Send;

    fn get_firmwares(&self) -> impl Future<Output = Result<Vec<FirmwareEntry>, Error>> + Send;

    fn upload_firmware(
        &self,
        blob: Vec<u8>,
        version: Version,
        restrict_nodes: Option<Vec<NodeId>>,
    ) -> impl Future<Output = Result<(), Error>> + Send;
}

impl DatabaseClient {
    #[tracing::instrument(name = "DatabaseClient::new()", level = "debug", err, skip_all)]
    pub async fn new(config: &Config) -> Result<Self, Error> {
        let node_id_cache = NodeIdCache::builder()
            .max_capacity(config.limits.devices.into()) // one per device
            .time_to_live(config.cache.auth_ttl)
            .async_eviction_listener(|k, v, c| {
                Box::pin(async move {
                    debug!("Auth cache evicted mapping '{k}'<=>'{v:?}': {c:?}");
                })
            })
            .build();
        let node_settings_cache = NodeSettingsCache::builder()
            .max_capacity(config.limits.devices.into()) // one per device
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
                backend: DynDatabaseBackend::new_box(
                    PostgresClient::new(
                        host,
                        *port,
                        user,
                        password,
                        name,
                        *ssl,
                        config.limits.devices,
                    )
                    .await?,
                ),
                node_id_cache,
                node_settings_cache,
            }),
            DatabaseConfig::Sqlite { file } => Ok(Self {
                backend: DynDatabaseBackend::new_box(
                    SqliteClient::new(file, config.limits.devices).await?,
                ),
                node_id_cache,
                node_settings_cache,
            }),
        }
    }

    pub async fn authorize_device(&self, mac: &Mac) -> Result<Option<NodeId>, Error> {
        if let Some(maybe_id) = self.node_id_cache.get(mac).await {
            debug!("Auth cache hit for '{mac}' -> '{maybe_id:?}'");
            return Ok(maybe_id);
        }

        debug!("Auth cache miss for '{mac}'");

        let maybe_id = self.backend.authorize_device(mac).await?;
        self.node_id_cache.insert(*mac, maybe_id).await;
        Ok(maybe_id)
    }

    pub async fn create_notification(&self, node_id: NodeId, content: &str) -> Result<(), Error> {
        self.backend.create_notification(node_id, content).await
    }

    pub async fn get_settings(&self, node_id: NodeId) -> Result<Option<NodeSettings>, Error> {
        if let Some(settings) = self.node_settings_cache.get(&node_id).await {
            debug!("Settings cache hit for '{node_id}'");
            return Ok(settings);
        }

        debug!("Settings cache miss for '{node_id}'");

        let settings = self.backend.get_settings(node_id).await?;
        self.node_settings_cache.insert(node_id, settings).await;
        Ok(settings)
    }

    pub async fn post_measurements(
        &self,
        node: NodeId,
        measurements: MeasurementCollection,
        wifi_stats: WiFiStats,
        os_version: Version,
        os_version_ext: Option<&str>,
    ) -> Result<(), Error> {
        self.backend
            .post_measurements(node, measurements, wifi_stats, os_version, os_version_ext)
            .await
    }

    pub async fn post_historic_measurements(
        &self,
        node: NodeId,
        measurements: Box<[HistoricMeasurement]>,
        wifi_stats: &WiFiStats,
        os_version: Version,
        os_version_ext: Option<&str>,
    ) -> Result<(), Error> {
        self.backend
            .post_historic_measurements(node, measurements, wifi_stats, os_version, os_version_ext)
            .await
    }

    pub async fn store_diagnostic_artifact(
        &self,
        node: NodeId,
        tag: &str,
        content: &str,
    ) -> Result<(), Error> {
        self.backend
            .store_diagnostic_artifact(node, tag, content)
            .await
    }

    pub async fn run_migrations(&self) -> Result<(), Error> {
        self.backend.run_migrations().await
    }

    pub async fn check_os_update(
        &self,
        node: NodeId,
        current_ver: Version,
    ) -> Result<Option<(Version, FirmwareBlob)>, Error> {
        self.backend.check_os_update(node, current_ver).await
    }

    pub async fn send_os_update_stat(
        &self,
        node_id: NodeId,
        old_ver: Version,
        new_ver: Version,
    ) -> Result<UpdateStatId, Error> {
        self.backend
            .send_os_update_stat(node_id, old_ver, new_ver)
            .await
    }

    pub async fn mark_os_update_stat(&self, node_id: NodeId, success: bool) -> Result<(), Error> {
        self.backend.mark_os_update_stat(node_id, success).await
    }

    pub async fn erase(&self, options: EraseOptions) -> Result<(), Error> {
        self.backend.erase(options).await
    }

    pub async fn get_firmwares(&self) -> Result<Vec<FirmwareEntry>, Error> {
        self.backend.get_firmwares().await
    }

    pub async fn upload_firmware(
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

/// Convert a temperature in decicelsius (as sent by nodes) to Celsius.
pub fn decicelsius_to_celsius(value: Temperature) -> f32 {
    f32::from(value) / 10.0
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

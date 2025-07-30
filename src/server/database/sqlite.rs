use std::str::FromStr;

use super::{FirmwareBlob, MeasurementId, NodeId, NodeSettings, UpdateStatId};
use crate::{error::Error, server::config::SqliteConfig};
use async_trait::async_trait;
use pwmp_client::pwmp_msg::{
    aliases::{AirPressure, BatteryVoltage, Humidity, Rssi, Temperature},
    mac::Mac,
    version::Version,
};
use sqlx::{
    Pool, Sqlite,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};

pub struct SqliteClient(Pool<Sqlite>);

impl SqliteClient {
    pub async fn new(config: &SqliteConfig) -> Result<Self, Error> {
        let opts = SqliteConnectOptions::from_str(&format!("sqlite://{}", config.path.display()))?;

        let pool = SqlitePoolOptions::new()
            .max_connections(3)
            .connect_with(opts)
            .await?;

        Ok(Self(pool))
    }
}

#[allow(unused_variables)]
#[async_trait]
impl super::Backend for SqliteClient {
    async fn setup_timezone(&self, tz: &str) -> Result<(), Error> {
        unimplemented!()
    }

    async fn authorize_device(&self, mac: &Mac) -> Result<Option<NodeId>, Error> {
        unimplemented!()
    }

    async fn create_notification(&self, node_id: NodeId, content: &str) -> Result<(), Error> {
        unimplemented!()
    }

    async fn get_settings(&self, node_id: NodeId) -> Result<Option<NodeSettings>, Error> {
        unimplemented!()
    }

    async fn post_results(
        &self,
        node: NodeId,
        temp: Temperature,
        hum: Humidity,
        air_p: Option<AirPressure>,
    ) -> Result<MeasurementId, Error> {
        unimplemented!()
    }

    async fn post_stats(
        &self,
        measurement: MeasurementId,
        battery: BatteryVoltage,
        wifi_ssid: &str,
        wifi_rssi: Rssi,
    ) -> Result<(), Error> {
        unimplemented!()
    }

    async fn run_migrations(&self) -> Result<(), Error> {
        unimplemented!()
    }

    async fn check_os_update(
        &self,
        node: NodeId,
        current_ver: Version,
    ) -> Result<Option<(Version, FirmwareBlob)>, Error> {
        unimplemented!()
    }

    async fn send_os_update_stat(
        &self,
        node_id: NodeId,
        old_ver: Version,
        new_ver: Version,
    ) -> Result<UpdateStatId, Error> {
        unimplemented!()
    }

    async fn mark_os_update_stat(&self, node_id: NodeId, success: bool) -> Result<(), Error> {
        unimplemented!()
    }

    async fn erase(&self, content_only: bool, keep_devices: bool) -> Result<(), Error> {
        unimplemented!()
    }
}

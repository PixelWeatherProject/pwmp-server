use super::{FirmwareBlob, MeasurementId, NodeId, NodeSettings, UpdateStatId};
use crate::{error::Error, server::config::PostgresConfig};
use async_trait::async_trait;
use pwmp_client::pwmp_msg::{
    aliases::{AirPressure, BatteryVoltage, Humidity, Rssi, Temperature},
    mac::Mac,
    version::Version,
};
use sqlx::{
    Pool, Postgres,
    postgres::{PgConnectOptions, PgPoolOptions, PgSslMode},
};

pub struct PostgresClient(Pool<Postgres>);

impl PostgresClient {
    pub async fn new(config: &PostgresConfig) -> Result<Self, Error> {
        let mut opts = PgConnectOptions::new()
            .host(&config.host)
            .port(config.port)
            .username(&config.user)
            .password(&config.password)
            .database(&config.name);

        if config.ssl {
            opts = opts.ssl_mode(PgSslMode::Require);
        }

        let pool = PgPoolOptions::new()
            .max_connections(3)
            .connect_with(opts)
            .await?;

        Ok(Self(pool))
    }

    #[tracing::instrument(
        name = "PostgresClient::get_supported_time_zones()",
        level = "debug",
        skip(self),
        err
    )]
    async fn get_supported_time_zones(&self) -> Result<Vec<String>, Error> {
        let results = sqlx::query("SELECT name FROM pg_timezone_names;")
            .fetch_all(&self.0)
            .await?;
        let names: Vec<String> = results.into_iter().filter_map(|record| record).collect();

        Ok(names)
    }

    #[tracing::instrument(
        name = "PostgresClient::validate_timezone()",
        level = "debug",
        skip(self, tz),
        err,
        ret /* this will print `tz` too */
    )]
    async fn validate_timezone<S: PartialEq<String> + std::fmt::Debug>(
        &self,
        tz: S,
    ) -> Result<bool, Error> {
        let supported = self.get_supported_time_zones().await?;

        Ok(supported.iter().any(|candidate| tz.eq(candidate)))
    }
}

#[allow(unused_variables)]
#[async_trait]
impl super::Backend for PostgresClient {
    #[tracing::instrument(
        name = "PostgresClient::setup_timezone()",
        level = "debug",
        skip(self),
        err
    )]
    async fn setup_timezone(&self, tz: &str) -> Result<(), Error> {
        if !self.validate_timezone(tz).await? {
            return Err(Error::InvalidTimeZone(tz.into()));
        }

        // This query needs to be dynamically generated.
        let sql = format!("SET TIME ZONE \"{tz}\"");
        sqlx::query(&sql).execute(&self.0).await?;

        Ok(())
    }

    #[tracing::instrument(
        name = "DatabaseClient::authorize_device()",
        level = "debug",
        skip(self),
        err,
        ret
    )]
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

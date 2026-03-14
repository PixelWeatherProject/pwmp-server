use super::config::Config;
use crate::{error::Error, server::config::DatabaseConfig};
use pwmp_client::pwmp_msg::{
    aliases::{AirPressure, BatteryVoltage, Humidity, Rssi, Temperature},
    mac::Mac,
    settings::NodeSettings,
    version::Version,
};
use sqlx::{
    Pool, Postgres, Sqlite,
    postgres::{PgConnectOptions, PgPoolOptions, PgSslMode},
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use std::path::PathBuf;

pub type NodeId = i32;
pub type MeasurementId = i32;
pub type FirmwareBlob = Box<[u8]>;
pub type UpdateStatId = i32;

pub enum DatabaseClient {
    Posgres(Pool<Postgres>),
    Sqlite(Pool<Sqlite>),
}

impl DatabaseClient {
    #[tracing::instrument(name = "DatabaseClient::init()", level = "debug", err, skip_all)]
    pub async fn new(config: &Config) -> Result<Self, Error> {
        match &config.database {
            DatabaseConfig::Postgres {
                host,
                port,
                user,
                password,
                name,
                ssl,
            } => Self::new_postgres(host, *port, user, password, name, *ssl).await,
            DatabaseConfig::Sqlite { file } => Self::new_sqlite(file).await,
        }
    }

    #[tracing::instrument(
        name = "DatabaseClient::authorize_device()",
        level = "debug",
        skip(self),
        err,
        ret
    )]
    pub async fn authorize_device(&self, mac: &Mac) -> Result<Option<NodeId>, Error> {
        let mac = mac.to_string();

        match self {
            Self::Posgres(pool) => _postgres_impl::authorize_device(pool, &mac).await,
            Self::Sqlite(pool) => _sqlite_impl::authorize_device(pool, &mac).await,
        }
    }

    #[tracing::instrument(
        name = "DatabaseClient::create_notification()",
        level = "debug",
        skip(self),
        err
    )]
    pub async fn create_notification(&self, node_id: NodeId, content: &str) -> Result<(), Error> {
        todo!()
    }

    #[tracing::instrument(
        name = "DatabaseClient::get_settings()",
        level = "debug",
        skip(self),
        err,
        ret
    )]
    pub async fn get_settings(&self, node_id: NodeId) -> Result<Option<NodeSettings>, Error> {
        todo!()
    }

    #[tracing::instrument(
        name = "DatabaseClient::post_results()",
        level = "debug",
        skip(self),
        err,
        ret
    )]
    pub async fn post_results(
        &self,
        node: NodeId,
        temp: Temperature,
        hum: Humidity,
        air_p: Option<AirPressure>,
    ) -> Result<MeasurementId, Error> {
        todo!()
    }

    #[tracing::instrument(
        name = "DatabaseClient::post_stats()",
        level = "debug",
        skip(self),
        err
    )]
    pub async fn post_stats(
        &self,
        measurement: MeasurementId,
        battery: BatteryVoltage,
        wifi_ssid: &str,
        wifi_rssi: Rssi,
    ) -> Result<(), Error> {
        todo!()
    }

    #[tracing::instrument(
        name = "DatabaseClient::run_migrations()",
        level = "debug",
        skip(self),
        err
    )]
    pub async fn run_migrations(&self) -> Result<(), Error> {
        todo!()
    }

    #[tracing::instrument(
        name = "DatabaseClient::check_os_update()",
        level = "debug",
        skip(self),
        err
    )]
    pub async fn check_os_update(
        &self,
        node: NodeId,
        current_ver: Version,
    ) -> Result<Option<(Version, FirmwareBlob)>, Error> {
        todo!()
    }

    #[tracing::instrument(
        name = "DatabaseClient::send_os_update_stat()",
        level = "debug",
        skip(self),
        err,
        ret
    )]
    pub async fn send_os_update_stat(
        &self,
        node_id: NodeId,
        old_ver: Version,
        new_ver: Version,
    ) -> Result<UpdateStatId, Error> {
        todo!()
    }

    #[tracing::instrument(
        name = "DatabaseClient::mark_os_update_stat()",
        level = "debug",
        skip(self),
        err
    )]
    pub async fn mark_os_update_stat(&self, node_id: NodeId, success: bool) -> Result<(), Error> {
        todo!()
    }

    #[tracing::instrument(name = "DatabaseClient::erase()", level = "debug", skip(self), err)]
    pub async fn erase(&self, content_only: bool, keep_devices: bool) -> Result<(), Error> {
        todo!()
    }

    async fn new_postgres(
        host: &str,
        port: u16,
        user: &str,
        password: &str,
        name: &str,
        ssl: bool,
    ) -> Result<Self, Error> {
        let mut opts = PgConnectOptions::new()
            .host(host)
            .port(port)
            .username(user)
            .password(password)
            .database(name);

        if ssl {
            opts = opts.ssl_mode(PgSslMode::Require);
        }

        let pool = PgPoolOptions::new()
            .max_connections(3)
            .connect_with(opts)
            .await?;

        Ok(Self::Posgres(pool))
    }

    async fn new_sqlite(path: &PathBuf) -> Result<Self, Error> {
        let opts = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(3)
            .connect_with(opts)
            .await?;

        Ok(Self::Sqlite(pool))
    }
}

mod _postgres_impl {
    use crate::{error::Error, server::db::NodeId};
    use sqlx::{Pool, Postgres};

    pub async fn authorize_device(
        pool: &Pool<Postgres>,
        mac: &str,
    ) -> Result<Option<NodeId>, Error> {
        Ok(
            sqlx::query_scalar(include_str!("../../queries/postgres/get_device_by_mac.sql"))
                .bind(mac)
                .fetch_optional(pool)
                .await?,
        )
    }
}

mod _sqlite_impl {
    use crate::{error::Error, server::db::NodeId};
    use sqlx::{Pool, Sqlite};

    pub async fn authorize_device(pool: &Pool<Sqlite>, mac: &str) -> Result<Option<NodeId>, Error> {
        Ok(
            sqlx::query_scalar("../../queries/sqlite/get_device_by_mac.sql")
                .bind(mac)
                .fetch_optional(pool)
                .await?,
        )
    }
}

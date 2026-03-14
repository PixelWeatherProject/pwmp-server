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
        match self {
            Self::Posgres(pool) => {
                _postgres_impl::create_notification(pool, node_id, content).await
            }
            Self::Sqlite(pool) => _sqlite_impl::create_notification(pool, node_id, content).await,
        }
    }

    #[tracing::instrument(
        name = "DatabaseClient::get_settings()",
        level = "debug",
        skip(self),
        err,
        ret
    )]
    pub async fn get_settings(&self, node_id: NodeId) -> Result<Option<NodeSettings>, Error> {
        match self {
            Self::Posgres(pool) => _postgres_impl::get_settings(pool, node_id).await,
            Self::Sqlite(pool) => _sqlite_impl::get_settings(pool, node_id).await,
        }
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
        match self {
            Self::Posgres(pool) => _postgres_impl::post_results(pool, node, temp, hum, air_p).await,
            Self::Sqlite(pool) => _sqlite_impl::post_results(pool, node, temp, hum, air_p).await,
        }
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
        match self {
            Self::Posgres(pool) => {
                _postgres_impl::post_stats(pool, measurement, battery, wifi_ssid, wifi_rssi).await
            }
            Self::Sqlite(pool) => {
                _sqlite_impl::post_stats(pool, measurement, battery, wifi_ssid, wifi_rssi).await
            }
        }
    }

    #[tracing::instrument(
        name = "DatabaseClient::run_migrations()",
        level = "debug",
        skip(self),
        err
    )]
    pub async fn run_migrations(&self) -> Result<(), Error> {
        match self {
            Self::Posgres(pool) => _postgres_impl::run_migrations(pool).await,
            Self::Sqlite(pool) => _sqlite_impl::run_migrations(pool).await,
        }
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
        match self {
            Self::Posgres(pool) => _postgres_impl::check_os_update(pool, node, current_ver).await,
            Self::Sqlite(pool) => _sqlite_impl::check_os_update(pool, node, current_ver).await,
        }
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
        match self {
            Self::Posgres(pool) => {
                _postgres_impl::send_os_update_stat(pool, node_id, old_ver, new_ver).await
            }
            Self::Sqlite(pool) => {
                _sqlite_impl::send_os_update_stat(pool, node_id, old_ver, new_ver).await
            }
        }
    }

    #[tracing::instrument(
        name = "DatabaseClient::mark_os_update_stat()",
        level = "debug",
        skip(self),
        err
    )]
    pub async fn mark_os_update_stat(&self, node_id: NodeId, success: bool) -> Result<(), Error> {
        match self {
            Self::Posgres(pool) => {
                _postgres_impl::mark_os_update_stat(pool, node_id, success).await
            }
            Self::Sqlite(pool) => _sqlite_impl::mark_os_update_stat(pool, node_id, success).await,
        }
    }

    #[tracing::instrument(name = "DatabaseClient::erase()", level = "debug", skip(self), err)]
    pub async fn erase(&self, content_only: bool, keep_devices: bool) -> Result<(), Error> {
        match self {
            Self::Posgres(pool) => _postgres_impl::erase(pool, content_only, keep_devices).await,
            Self::Sqlite(pool) => _sqlite_impl::erase(pool, content_only, keep_devices).await,
        }
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
    use crate::{
        error::Error,
        server::db::{FirmwareBlob, MeasurementId, NodeId, UpdateStatId},
    };
    use pwmp_client::pwmp_msg::{
        aliases::{AirPressure, BatteryVoltage, Humidity, Rssi, Temperature},
        settings::NodeSettings,
        version::Version,
    };
    use sqlx::{Pool, Postgres, Row};

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

    pub async fn create_notification(
        pool: &Pool<Postgres>,
        node_id: NodeId,
        content: &str,
    ) -> Result<(), Error> {
        sqlx::query(include_str!(
            "../../queries/postgres/create_notification.sql"
        ))
        .bind(node_id)
        .bind(content)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn get_settings(
        pool: &Pool<Postgres>,
        node_id: NodeId,
    ) -> Result<Option<NodeSettings>, Error> {
        let result = sqlx::query(include_str!(
            "../../queries/postgres/get_device_settings.sql"
        ))
        .bind(node_id)
        .fetch_optional(pool)
        .await?;

        let result = match result {
            Some(row) => Some(NodeSettings {
                battery_ignore: row.get(0),
                ota: row.get(1),
                sleep_time: row.get::<i32, _>(2).try_into()?,
                sbop: row.get(3),
                mute_notifications: row.get(4),
            }),
            None => None,
        };

        Ok(result)
    }

    pub async fn post_results(
        pool: &Pool<Postgres>,
        node: NodeId,
        temp: Temperature,
        hum: Humidity,
        air_p: Option<AirPressure>,
    ) -> Result<MeasurementId, Error> {
        let signed_air_p: Option<i16> = match air_p {
            Some(value) => Some(value.try_into()?),
            None => None,
        };

        let result = sqlx::query_scalar(include_str!("../../queries/postgres/post_results.sql"))
            .bind(node)
            .bind(temp)
            .bind(i16::from(hum))
            .bind(signed_air_p)
            .fetch_one(pool)
            .await?;

        Ok(result)
    }

    pub async fn post_stats(
        pool: &Pool<Postgres>,
        measurement: MeasurementId,
        battery: BatteryVoltage,
        wifi_ssid: &str,
        wifi_rssi: Rssi,
    ) -> Result<(), Error> {
        sqlx::query(include_str!("../../queries/postgres/post_stats.sql"))
            .bind(measurement)
            .bind(battery)
            .bind(wifi_ssid)
            .bind(i16::from(wifi_rssi))
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn run_migrations(pool: &Pool<Postgres>) -> Result<(), Error> {
        sqlx::raw_sql(include_str!("../../queries/postgres/migrate.sql"))
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn check_os_update(
        pool: &Pool<Postgres>,
        node: NodeId,
        current_ver: Version,
    ) -> Result<Option<(Version, FirmwareBlob)>, Error> {
        let (version_major, version_middle, version_minor) = current_ver.to_signed_triple();

        let result = sqlx::query(include_str!("../../queries/postgres/get_os_update.sql"))
            .bind(node)
            .bind(version_major)
            .bind(version_middle)
            .bind(version_minor)
            .fetch_optional(pool)
            .await?;

        match result {
            Some(row) => {
                let new_version = Version::new(
                    row.get::<i8, _>(0).try_into()?,
                    row.get::<i8, _>(1).try_into()?,
                    row.get::<i8, _>(3).try_into()?,
                );
                let blob = row.get::<Vec<u8>, _>(3).into_boxed_slice();
                Ok(Some((new_version, blob)))
            }
            None => Ok(None),
        }
    }

    pub async fn send_os_update_stat(
        pool: &Pool<Postgres>,
        node_id: NodeId,
        old_ver: Version,
        new_ver: Version,
    ) -> Result<UpdateStatId, Error> {
        let (old_version_major, old_version_middle, old_version_minor) = old_ver.to_signed_triple();
        let (new_version_major, new_version_middle, new_version_minor) = new_ver.to_signed_triple();

        let result = sqlx::query_scalar(include_str!(
            "../../queries/postgres/send_os_update_event.sql"
        ))
        .bind(node_id)
        .bind(old_version_major)
        .bind(old_version_middle)
        .bind(old_version_minor)
        .bind(new_version_major)
        .bind(new_version_middle)
        .bind(new_version_minor)
        .fetch_one(pool)
        .await?;

        Ok(result)
    }

    pub async fn mark_os_update_stat(
        pool: &Pool<Postgres>,
        node_id: NodeId,
        success: bool,
    ) -> Result<(), Error> {
        todo!()
    }

    pub async fn erase(
        pool: &Pool<Postgres>,
        content_only: bool,
        keep_devices: bool,
    ) -> Result<(), Error> {
        todo!()
    }
}

mod _sqlite_impl {
    use crate::{
        error::Error,
        server::db::{FirmwareBlob, MeasurementId, NodeId, UpdateStatId},
    };
    use pwmp_client::pwmp_msg::{
        aliases::{AirPressure, BatteryVoltage, Humidity, Rssi, Temperature},
        settings::NodeSettings,
        version::Version,
    };
    use sqlx::{Pool, Row, Sqlite};

    pub async fn authorize_device(pool: &Pool<Sqlite>, mac: &str) -> Result<Option<NodeId>, Error> {
        Ok(
            sqlx::query_scalar(include_str!("../../queries/sqlite/get_device_by_mac.sql"))
                .bind(mac)
                .fetch_optional(pool)
                .await?,
        )
    }

    pub async fn create_notification(
        pool: &Pool<Sqlite>,
        node_id: NodeId,
        content: &str,
    ) -> Result<(), Error> {
        sqlx::query(include_str!("../../queries/sqlite/create_notification.sql"))
            .bind(node_id)
            .bind(content)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn get_settings(
        pool: &Pool<Sqlite>,
        node_id: NodeId,
    ) -> Result<Option<NodeSettings>, Error> {
        let result = sqlx::query(include_str!("../../queries/sqlite/get_device_settings.sql"))
            .bind(node_id)
            .fetch_optional(pool)
            .await?;

        let result = match result {
            Some(row) => Some(NodeSettings {
                battery_ignore: row.get(0),
                ota: row.get(1),
                sleep_time: row.get::<i32, _>(2).try_into()?,
                sbop: row.get(3),
                mute_notifications: row.get(4),
            }),
            None => None,
        };

        Ok(result)
    }

    pub async fn post_results(
        pool: &Pool<Sqlite>,
        node: NodeId,
        temp: Temperature,
        hum: Humidity,
        air_p: Option<AirPressure>,
    ) -> Result<MeasurementId, Error> {
        let result = sqlx::query_scalar(include_str!("../../queries/sqlite/post_results.sql"))
            .bind(node)
            .bind(temp)
            .bind(hum)
            .bind(air_p)
            .fetch_one(pool)
            .await?;

        Ok(result)
    }

    pub async fn post_stats(
        pool: &Pool<Sqlite>,
        measurement: MeasurementId,
        battery: BatteryVoltage,
        wifi_ssid: &str,
        wifi_rssi: Rssi,
    ) -> Result<(), Error> {
        sqlx::query(include_str!("../../queries/sqlite/post_stats.sql"))
            .bind(measurement)
            .bind(battery)
            .bind(wifi_ssid)
            .bind(wifi_rssi)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn run_migrations(pool: &Pool<Sqlite>) -> Result<(), Error> {
        sqlx::raw_sql(include_str!("../../queries/sqlite/migrate.sql"))
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn check_os_update(
        pool: &Pool<Sqlite>,
        node: NodeId,
        current_ver: Version,
    ) -> Result<Option<(Version, FirmwareBlob)>, Error> {
        todo!()
    }

    pub async fn send_os_update_stat(
        pool: &Pool<Sqlite>,
        node_id: NodeId,
        old_ver: Version,
        new_ver: Version,
    ) -> Result<UpdateStatId, Error> {
        todo!()
    }

    pub async fn mark_os_update_stat(
        pool: &Pool<Sqlite>,
        node_id: NodeId,
        success: bool,
    ) -> Result<(), Error> {
        todo!()
    }

    pub async fn erase(
        pool: &Pool<Sqlite>,
        content_only: bool,
        keep_devices: bool,
    ) -> Result<(), Error> {
        todo!()
    }
}

use super::{EraseOptions, FirmwareBlob, MeasurementId, NodeId, SleepTime, UpdateStatId};
use crate::error::Error;
use pwmp_client::pwmp_msg::{
    aliases::{AirPressure, BatteryVoltage, Humidity, Rssi, Temperature},
    mac::Mac,
    settings::NodeSettings,
    version::Version,
};
use sqlx::{
    Pool, Postgres, Row,
    postgres::{PgConnectOptions, PgPoolOptions, PgSslMode},
};

pub struct PostgresClient(Pool<Postgres>);

impl PostgresClient {
    #[tracing::instrument(name = "PostgresClient::new()", level = "debug", err, skip_all)]
    pub async fn new(
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

        Ok(Self(pool))
    }

    pub async fn get_last_os_update_stat_for_node(
        &self,
        node_id: NodeId,
    ) -> Result<UpdateStatId, Error> {
        let row = sqlx::query(include_str!(
            "../../../queries/postgres/get_last_update_event.sql"
        ))
        .bind(node_id)
        .fetch_one(&self.0)
        .await?;

        Ok(row.get(0))
    }
}

impl super::DatabaseBackend for PostgresClient {
    #[tracing::instrument(
        name = "PostgresClient::authorize_device()",
        level = "debug",
        skip(self),
        err,
        ret
    )]
    async fn authorize_device(&self, mac: &Mac) -> Result<Option<NodeId>, Error> {
        let mac = mac.to_string();

        let id = sqlx::query_scalar(include_str!(
            "../../../queries/postgres/get_device_by_mac.sql"
        ))
        .bind(mac)
        .fetch_optional(&self.0)
        .await?;

        Ok(id)
    }

    #[tracing::instrument(
        name = "PostgresClient::create_notification()",
        level = "debug",
        skip(self),
        err
    )]
    async fn create_notification(&self, node_id: NodeId, content: &str) -> Result<(), Error> {
        sqlx::query(include_str!(
            "../../../queries/postgres/create_notification.sql"
        ))
        .bind(node_id)
        .bind(content)
        .execute(&self.0)
        .await?;
        Ok(())
    }

    #[tracing::instrument(
        name = "PostgresClient::get_settings()",
        level = "debug",
        skip(self),
        err,
        ret
    )]
    async fn get_settings(&self, node_id: NodeId) -> Result<Option<NodeSettings>, Error> {
        let result = sqlx::query(include_str!(
            "../../../queries/postgres/get_device_settings.sql"
        ))
        .bind(node_id)
        .fetch_optional(&self.0)
        .await?;

        let result = match result {
            Some(row) => Some(NodeSettings {
                battery_ignore: row.get(0),
                ota: row.get(1),
                sleep_time: row.get::<SleepTime, _>(2).try_into()?,
                sbop: row.get(3),
                mute_notifications: row.get(4),
            }),
            None => None,
        };

        Ok(result)
    }

    #[tracing::instrument(
        name = "PostgresClient::post_results()",
        level = "debug",
        skip(self),
        err,
        ret
    )]
    async fn post_results(
        &self,
        node: NodeId,
        temp: Temperature,
        hum: Humidity,
        air_p: Option<AirPressure>,
    ) -> Result<MeasurementId, Error> {
        let signed_air_p: Option<i16> = match air_p {
            Some(value) => Some(value.try_into()?),
            None => None,
        };

        let result = sqlx::query_scalar(include_str!("../../../queries/postgres/post_results.sql"))
            .bind(node)
            .bind(temp)
            .bind(i16::from(hum))
            .bind(signed_air_p)
            .fetch_one(&self.0)
            .await?;

        Ok(result)
    }

    #[tracing::instrument(
        name = "PostgresClient::post_stats()",
        level = "debug",
        skip(self),
        err
    )]
    async fn post_stats(
        &self,
        measurement: super::MeasurementId,
        battery: BatteryVoltage,
        wifi_ssid: &str,
        wifi_rssi: Rssi,
    ) -> Result<(), Error> {
        sqlx::query(include_str!("../../../queries/postgres/post_stats.sql"))
            .bind(measurement)
            .bind(battery)
            .bind(wifi_ssid)
            .bind(i16::from(wifi_rssi))
            .execute(&self.0)
            .await?;
        Ok(())
    }

    #[tracing::instrument(
        name = "PostgresClient::run_migrations()",
        level = "debug",
        skip(self),
        err
    )]
    async fn run_migrations(&self) -> Result<(), Error> {
        sqlx::raw_sql(include_str!("../../../queries/postgres/migrate.sql"))
            .execute(&self.0)
            .await?;
        Ok(())
    }

    #[tracing::instrument(
        name = "PostgresClient::check_os_update()",
        level = "debug",
        skip(self),
        err
    )]
    async fn check_os_update(
        &self,
        node: NodeId,
        current_ver: Version,
    ) -> Result<Option<(Version, FirmwareBlob)>, Error> {
        let (version_major, version_middle, version_minor) = current_ver.to_signed_triple();

        let result = sqlx::query(include_str!("../../../queries/postgres/get_os_update.sql"))
            .bind(node)
            .bind(version_major)
            .bind(version_middle)
            .bind(version_minor)
            .fetch_optional(&self.0)
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

    #[tracing::instrument(
        name = "PostgresClient::send_os_update_stat()",
        level = "debug",
        skip(self),
        err,
        ret
    )]
    async fn send_os_update_stat(
        &self,
        node_id: NodeId,
        old_ver: Version,
        new_ver: Version,
    ) -> Result<UpdateStatId, Error> {
        let (old_version_major, old_version_middle, old_version_minor) = old_ver.to_signed_triple();
        let (new_version_major, new_version_middle, new_version_minor) = new_ver.to_signed_triple();

        let result = sqlx::query_scalar(include_str!(
            "../../../queries/postgres/send_os_update_event.sql"
        ))
        .bind(node_id)
        .bind(old_version_major)
        .bind(old_version_middle)
        .bind(old_version_minor)
        .bind(new_version_major)
        .bind(new_version_middle)
        .bind(new_version_minor)
        .fetch_one(&self.0)
        .await?;

        Ok(result)
    }

    #[tracing::instrument(
        name = "PostgresClient::mark_os_update_stat()",
        level = "debug",
        skip(self),
        err
    )]
    async fn mark_os_update_stat(&self, node_id: NodeId, success: bool) -> Result<(), Error> {
        let update_stat_id = self.get_last_os_update_stat_for_node(node_id).await?;

        sqlx::query(include_str!(
            "../../../queries/postgres/update_os_update_event.sql"
        ))
        .bind(update_stat_id)
        .bind(success)
        .execute(&self.0)
        .await?;

        Ok(())
    }

    #[tracing::instrument(name = "PostgresClient::erase()", level = "debug", skip(self), err)]
    async fn erase(&self, options: EraseOptions) -> Result<(), Error> {
        let sql = match options {
            EraseOptions::Everything => {
                include_str!("../../../queries/postgres/erase_database.sql")
            }
            EraseOptions::ContentOnly { keep_devices } => {
                if keep_devices {
                    include_str!(
                        "../../../queries/postgres/erase_database_content_keep_devices_and_settings.sql"
                    )
                } else {
                    include_str!("../../../queries/postgres/erase_database_content.sql")
                }
            }
        };

        sqlx::query(sql).execute(&self.0).await?;
        Ok(())
    }
}

use super::{FirmwareBlob, MeasurementId, NodeId, NodeSettings, UpdateStatId};
use crate::{error::Error, server::config::PostgresConfig};
use async_trait::async_trait;
use pwmp_client::pwmp_msg::{
    aliases::{AirPressure, BatteryVoltage, Humidity, Rssi, Temperature},
    mac::Mac,
    version::Version,
};
use sqlx::{
    Pool, Postgres, Row,
    postgres::{PgConnectOptions, PgPoolOptions, PgSslMode},
};
use tracing::{debug, warn};

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
        let names: Vec<String> = results
            .into_iter()
            .filter_map(|record| record.try_get("name").ok())
            .collect();

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

    async fn get_latest_fw_for_node(
        &self,
        node_id: NodeId,
    ) -> Result<Option<(Version, i32)>, Error> {
        let row = sqlx::query(
            "SELECT id, verson_major, version_middle, version_minor
                 FROM firmwares
                 WHERE restrict_nodes IS NULL
                 OR $1 = ANY(restrict_nodes)
                 ORDER BY added_date LIMIT 1",
        )
        .bind(node_id)
        .fetch_one(&self.0)
        .await;

        match row {
            Ok(row) => {
                let fw_id = row.try_get("id")?;
                let version_major = row.try_get::<i8, _>("version_major")?.try_into()?;
                let version_middle = row.try_get::<i8, _>("version_middle")?.try_into()?;
                let version_minor = row.try_get::<i8, _>("version_minor")?.try_into()?;

                Ok(Some((
                    Version::new(version_major, version_middle, version_minor),
                    fw_id,
                )))
            }
            Err(sqlx::Error::RowNotFound) => Ok(None),
            Err(why) => Err(Error::Database(why)),
        }
    }

    async fn get_last_os_update_stat(&self, node: NodeId) -> Result<UpdateStatId, Error> {
        sqlx::query("")
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
        name = "PostgresClient::authorize_device()",
        level = "debug",
        skip(self),
        err,
        ret
    )]
    async fn authorize_device(&self, mac: &Mac) -> Result<Option<NodeId>, Error> {
        let mac = mac.to_string();

        let result = sqlx::query("SELECT devices.id FROM devices WHERE mac_address = $1")
            .bind(mac)
            .fetch_one(&self.0)
            .await;

        match result {
            Ok(res) => Ok(Some(res.try_get::<NodeId, _>("devices.id")?)),
            Err(sqlx::Error::RowNotFound) => Ok(None),
            Err(why) => Err(Error::Database(why)),
        }
    }

    #[tracing::instrument(
        name = "PostgresClient::create_notification()",
        level = "debug",
        skip(self),
        err
    )]
    async fn create_notification(&self, node_id: NodeId, content: &str) -> Result<(), Error> {
        sqlx::query("INSERT INTO notifications(node, content) VALUES ($1, $2)")
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
        let result = sqlx::query("SELECT battery_ignore, ota, sleep_time, sbop, mute_notifications FROM settings WHERE node = $1").bind(node_id).fetch_one(&self.0).await;

        match result {
            Ok(record) => Ok(Some(NodeSettings {
                battery_ignore: record.try_get("battery_ignore")?,
                ota: record.try_get("ota")?,
                sleep_time: record.try_get::<i16, _>("sleep_time")?.try_into()?,
                sbop: record.try_get("sbop")?,
                mute_notifications: record.try_get("mute_notifications")?,
            })),
            Err(sqlx::error::Error::RowNotFound) => Ok(None),
            Err(why) => Err(why.into()),
        }
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

        let result = sqlx::query(
            r#"INSERT INTO measurements(
                "node",
                "temperature",
                "humidity",
                "air_pressure"
                )
            VALUES ($1, $2, $3, $4) RETURNING id"#,
        )
        .bind(node)
        .bind(temp)
        .bind(signed_air_p)
        .fetch_one(&self.0)
        .await?;

        Ok(result.try_get("id")?)
    }

    #[tracing::instrument(
        name = "PostgresClient::post_stats()",
        level = "debug",
        skip(self),
        err
    )]
    async fn post_stats(
        &self,
        measurement: MeasurementId,
        battery: BatteryVoltage,
        wifi_ssid: &str,
        wifi_rssi: Rssi,
    ) -> Result<(), Error> {
        sqlx::query(
            r#"INSERT INTO statistics(
                "measurement",
                "battery",
                "wifi_ssid",
                "wifi_rssi"
            )
            VALUES ($1, $2, $3, $4)"#,
        )
        .bind(measurement)
        .bind(battery)
        .bind(wifi_ssid)
        .bind(wifi_rssi)
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
        unimplemented!()
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
        let Some((version, fw_id)) = self.get_latest_fw_for_node(node).await? else {
            warn!("No firmwares available for node #{node}, unable to check for updates");
            return Ok(None);
        };

        debug!("Latest firmware for node #{node} is {version}");
        if current_ver == version {
            debug!("Node #{node} is up to date");
            return Ok(None);
        } else {
            debug!("Node #{node} out of date");
        }

        let fw_blob = sqlx::query("SELECT firmware_blob FROM firmwares WHERE id = $1")
            .bind(fw_id)
            .fetch_one(&self.0)
            .await?
            .try_get::<Vec<u8>, _>("firmware_blob")?;

        Ok(Some((version, fw_blob.into_boxed_slice())))
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
        Ok(sqlx::query(
            r#"
            INSERT INTO firmware_stats (
                node,
                from_version_major,
                from_version_middle,
                from_version_minor,
                to_version_major,
                to_version_middle,
                to_version_minor
            )
            VALUES
                ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id;
            "#,
        )
        .bind(node_id)
        .bind::<i8>(old_ver.major().try_into()?)
        .bind::<i8>(old_ver.middle().try_into()?)
        .bind::<i8>(old_ver.minor().try_into()?)
        .bind::<i8>(new_ver.major().try_into()?)
        .bind::<i8>(new_ver.middle().try_into()?)
        .bind::<i8>(new_ver.minor().try_into()?)
        .fetch_one(&self.0)
        .await?
        .try_get("id")?)
    }

    #[tracing::instrument(
        name = "PostgresClient::mark_os_update_stat()",
        level = "debug",
        skip(self),
        err
    )]
    async fn mark_os_update_stat(&self, node_id: NodeId, success: bool) -> Result<(), Error> {
        let last_update_id = self.get_last_os_update_stat(node_id).await?;
        query!(
            self.0,
            "queries/update_os_update_event.sql",
            execute,
            last_update_id,
            success
        )?;

        Ok(())
    }

    async fn erase(&self, content_only: bool, keep_devices: bool) -> Result<(), Error> {
        unimplemented!()
    }
}

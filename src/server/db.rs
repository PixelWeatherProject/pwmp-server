use super::config::Config;
use crate::error::Error;
use log::error;
use pwmp_client::pwmp_msg::{
    aliases::{AirPressure, BatteryVoltage, Humidity, Rssi, Temperature},
    mac::Mac,
    settings::NodeSettings,
    version::Version,
};
use sqlx::{
    Pool, Postgres,
    postgres::{PgConnectOptions, PgPoolOptions, PgSslMode},
};

pub type NodeId = i32;
pub type MeasurementId = i32;
pub type FirmwareBlob = Box<[u8]>;
pub type UpdateStatId = i32;

pub struct DatabaseClient(Pool<Postgres>);

macro_rules! query {
    ($pool: expr, $qfile: literal, $method: ident, $($bindings: tt)*) => {
        sqlx::query_file!($qfile, $($bindings)*).$method(&$pool).await
    };
}

impl DatabaseClient {
    pub async fn new(config: &Config) -> Result<Self, Error> {
        let mut opts = PgConnectOptions::new()
            .host(&config.database.host)
            .port(config.database.port)
            .username(&config.database.user)
            .password(&config.database.password)
            .database(&config.database.name);

        if config.database.ssl {
            opts = opts.ssl_mode(PgSslMode::Require);
        }

        let pool = PgPoolOptions::new()
            .max_connections(3)
            .connect_with(opts)
            .await?;

        Ok(Self(pool))
    }

    pub async fn setup_timezone(&self, tz: &str) -> Result<(), Error> {
        if !self.validate_timezone(tz).await? {
            error!("Timezone \"{tz}\" is not supported by database, leaving defaults");
            return Ok(());
        }

        // This query needs to be dynamically generated.
        let sql = format!("SET TIME ZONE \"{tz}\"");
        sqlx::query(&sql).execute(&self.0).await?;

        Ok(())
    }

    pub async fn authorize_device(&self, mac: &Mac) -> Result<Option<NodeId>, Error> {
        let mac = mac.to_string();

        let result = query!(self.0, "queries/get_device_by_mac.sql", fetch_one, mac);
        match result {
            Ok(res) => Ok(Some(res.id)),
            Err(sqlx::Error::RowNotFound) => Ok(None),
            Err(why) => Err(Error::Database(why)),
        }
    }

    pub async fn create_notification(&self, node_id: NodeId, content: &str) -> Result<(), Error> {
        query!(
            self.0,
            "queries/create_notification.sql",
            execute,
            node_id,
            content,
        )?;

        Ok(())
    }

    pub async fn get_settings(&self, node_id: NodeId) -> Result<Option<NodeSettings>, Error> {
        let result = query!(
            self.0,
            "queries/get_device_settings.sql",
            fetch_one,
            node_id
        );

        match result {
            Ok(record) => Ok(Some(NodeSettings {
                battery_ignore: record.battery_ignore,
                ota: record.ota,
                sleep_time: record.sleep_time.try_into()?,
                sbop: record.sbop,
                mute_notifications: record.mute_notifications,
            })),
            Err(sqlx::error::Error::RowNotFound) => Ok(None),
            Err(why) => Err(why.into()),
        }
    }

    pub async fn post_results(
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

        let result = query!(
            self.0,
            "queries/post_results.sql",
            fetch_one,
            node,
            temp,
            i16::from(hum),
            signed_air_p
        )?;

        Ok(result.id)
    }

    pub async fn post_stats(
        &self,
        measurement: MeasurementId,
        battery: BatteryVoltage,
        wifi_ssid: &str,
        wifi_rssi: Rssi,
    ) -> Result<(), Error> {
        query!(
            self.0,
            "queries/post_stats.sql",
            execute,
            measurement,
            battery,
            wifi_ssid,
            i16::from(wifi_rssi)
        )?;

        Ok(())
    }

    pub async fn run_migrations(&self) -> Result<(), Error> {
        crate::MIGRATOR.run(&self.0).await?;

        Ok(())
    }

    pub async fn check_os_update(
        &self,
        node: NodeId,
        current_ver: Version,
    ) -> Result<Option<(Version, FirmwareBlob)>, Error> {
        let (version_major, version_middle, version_minor) = current_ver.to_signed_triple();
        let result = query!(
            self.0,
            "queries/get_os_update.sql",
            fetch_one,
            node,
            version_major,
            version_middle,
            version_minor
        );

        match result {
            Ok(update_info) => {
                let version = Version::new(
                    update_info.version_major.try_into()?,
                    update_info.version_middle.try_into()?,
                    update_info.version_minor.try_into()?,
                );

                Ok(Some((version, update_info.firmware.into_boxed_slice())))
            }
            Err(sqlx::Error::RowNotFound) => Ok(None),
            Err(why) => Err(Error::Database(why)),
        }
    }

    pub async fn send_os_update_stat(
        &self,
        node_id: NodeId,
        old_ver: Version,
        new_ver: Version,
    ) -> Result<UpdateStatId, Error> {
        let (old_version_major, old_version_middle, old_version_minor) = old_ver.to_signed_triple();
        let (new_version_major, new_version_middle, new_version_minor) = new_ver.to_signed_triple();

        let result = query!(
            self.0,
            "queries/send_os_update_event.sql",
            fetch_one,
            node_id,
            old_version_major,
            old_version_middle,
            old_version_minor,
            new_version_major,
            new_version_middle,
            new_version_minor
        )?;

        Ok(result.id)
    }

    pub async fn mark_os_update_stat(&self, node_id: NodeId, success: bool) -> Result<(), Error> {
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

    pub async fn erase(&self, content_only: bool, keep_devices: bool) -> Result<(), Error> {
        if content_only {
            if keep_devices {
                query!(
                    self.0,
                    "queries/erase_database_content_keep_devices_and_settings.sql",
                    execute,
                )?;
            } else {
                query!(self.0, "queries/erase_database_content.sql", execute,)?;
            }

            query!(self.0, "queries/drop_migrations_table.sql", execute,)?;
        } else if keep_devices {
            query!(
                self.0,
                "queries/erase_database_content_keep_devices_and_settings.sql",
                execute,
            )?;
        } else {
            query!(self.0, "queries/erase_database.sql", execute,)?;
        }

        Ok(())
    }

    async fn get_last_os_update_stat(&self, node_id: NodeId) -> Result<UpdateStatId, Error> {
        Ok(query!(
            self.0,
            "queries/get_last_update_event.sql",
            fetch_one,
            node_id
        )?
        .id)
    }

    async fn get_supported_time_zones(&self) -> Result<Vec<String>, Error> {
        let results = query!(self.0, "queries/get_tz_names.sql", fetch_all,)?;
        let names: Vec<String> = results
            .into_iter()
            .filter_map(|record| record.name)
            .collect();

        Ok(names)
    }

    async fn validate_timezone<S: PartialEq<String>>(&self, tz: S) -> Result<bool, Error> {
        let supported = self.get_supported_time_zones().await?;

        Ok(supported.iter().any(|candidate| tz.eq(candidate)))
    }
}

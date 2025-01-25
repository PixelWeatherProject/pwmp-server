use super::config::Config;
use crate::error::Error;
use pwmp_msg::{
    aliases::{AirPressure, BatteryVoltage, Humidity, Rssi, Temperature},
    mac::Mac,
    settings::NodeSettings,
    version::Version,
};
use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions, PgSslMode},
    Pool, Postgres,
};
use tokio::runtime::Runtime;

pub type NodeId = i16;
pub type MeasurementId = i32;
pub type FirmwareBlob = Box<[u8]>;
pub type UpdateStatId = i32;

pub struct DatabaseClient {
    rt: Runtime,
    pool: Pool<Postgres>,
}

macro_rules! query {
    ($runtime: expr, $pool: expr, $qfile: literal, $method: ident, $($bindings: tt)*) => {
        $runtime.block_on(async {
            sqlx::query_file!($qfile, $($bindings)*).$method(&$pool).await
        })
    };
}

impl DatabaseClient {
    pub fn new(config: &Config) -> Result<Self, Error> {
        let rt = Runtime::new()?;
        let mut opts = PgConnectOptions::new()
            .host(&config.database.host)
            .port(config.database.port)
            .username(&config.database.user)
            .password(&config.database.password)
            .database(&config.database.name);

        if config.database.ssl {
            opts = opts.ssl_mode(PgSslMode::Require);
        }

        let pool = rt.block_on(async {
            PgPoolOptions::new()
                .max_connections(3)
                .connect_with(opts)
                .await
        })?;

        Ok(Self { rt, pool })
    }

    pub fn authorize_device(&self, mac: &Mac) -> Result<Option<NodeId>, Error> {
        let mac = mac.to_string();

        let result = query!(
            self.rt,
            self.pool,
            "queries/get_device_by_mac.sql",
            fetch_one,
            mac
        );

        match result {
            Ok(res) => Ok(Some(res.id)),
            Err(sqlx::Error::RowNotFound) => Ok(None),
            Err(why) => Err(Error::Database(why)),
        }
    }

    pub fn create_notification(&self, node_id: NodeId, content: &str) -> Result<(), Error> {
        query!(
            self.rt,
            self.pool,
            "queries/create_notification.sql",
            execute,
            node_id,
            content,
        )?;

        Ok(())
    }

    pub fn get_settings(&self, node_id: NodeId) -> Result<Option<NodeSettings>, Error> {
        let result = query!(
            self.rt,
            self.pool,
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

    pub fn post_results(
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
            self.rt,
            self.pool,
            "queries/post_results.sql",
            fetch_one,
            node,
            temp,
            i16::from(hum),
            signed_air_p
        )?;

        Ok(result.id)
    }

    pub fn post_stats(
        &self,
        measurement: MeasurementId,
        battery: &BatteryVoltage,
        wifi_ssid: &str,
        wifi_rssi: Rssi,
    ) -> Result<(), Error> {
        query!(
            self.rt,
            self.pool,
            "queries/post_stats.sql",
            execute,
            measurement,
            battery,
            wifi_ssid,
            i16::from(wifi_rssi)
        )?;

        Ok(())
    }

    pub fn run_migrations(&self) -> Result<(), Error> {
        self.rt
            .block_on(async { crate::MIGRATOR.run(&self.pool).await })?;

        Ok(())
    }

    pub fn check_os_update(
        &self,
        node: NodeId,
        current_ver: Version,
    ) -> Result<Option<(Version, FirmwareBlob)>, Error> {
        let (version_major, version_middle, version_minor) = current_ver.to_signed_triple();
        let result = query!(
            self.rt,
            self.pool,
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

    pub fn send_os_update_stat(
        &self,
        node_id: NodeId,
        old_ver: Version,
        new_ver: Version,
    ) -> Result<UpdateStatId, Error> {
        let (old_version_major, old_version_middle, old_version_minor) = old_ver.to_signed_triple();
        let (new_version_major, new_version_middle, new_version_minor) = new_ver.to_signed_triple();

        let result = query!(
            self.rt,
            self.pool,
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

    pub fn mark_os_update_stat(&self, node_id: NodeId, success: bool) -> Result<(), Error> {
        let last_update_id = self.get_last_os_update_stat(node_id)?;

        query!(
            self.rt,
            self.pool,
            "queries/update_os_update_event.sql",
            execute,
            last_update_id,
            success
        )?;

        Ok(())
    }

    pub fn erase(&self) -> Result<(), Error> {
        query!(self.rt, self.pool, "queries/erase_database.sql", execute,)?;
        Ok(())
    }

    fn get_last_os_update_stat(&self, node_id: NodeId) -> Result<UpdateStatId, Error> {
        Ok(query!(
            self.rt,
            self.pool,
            "queries/get_last_update_event.sql",
            fetch_one,
            node_id
        )?
        .id)
    }
}

use super::config::Config;
use crate::error::Error;
use postgres::{Client as PostgresClient, NoTls, error::SqlState};
use pwmp_client::pwmp_msg::{
    aliases::{AirPressure, BatteryVoltage, Humidity, Rssi, Temperature},
    mac::Mac,
    settings::NodeSettings,
    version::Version,
};

pub type NodeId = i16;
pub type MeasurementId = i32;
pub type FirmwareBlob = Box<[u8]>;
pub type UpdateStatId = i32;

pub struct DatabaseClient(PostgresClient);

impl DatabaseClient {
    pub fn new(config: &Config) -> Result<Self, Error> {
        let params = format!(
            "host={} port={} user={} password='{}' dbname='{}' sslmode='{}' connect_timeout=4",
            config.database.host,
            config.database.port,
            config.database.user,
            config.database.password,
            config.database.name,
            if config.database.ssl {
                "prefer"
            } else {
                "disable"
            }
        );

        let client = PostgresClient::connect(&params, NoTls)?;
        Ok(Self(client))
    }

    pub fn authorize_device(&mut self, mac: &Mac) -> Result<Option<NodeId>, Error> {
        let mac = mac.to_string();

        let result = self
            .0
            .query_one(include_str!("../../queries/get_device_by_mac.sql"), &[&mac]);

        match result {
            Ok(res) => Ok(Some(res.try_get("id")?)),
            Err(e) if e.code() == Some(&SqlState::TOO_MANY_ROWS) => Ok(None),
            Err(why) => Err(Error::Database(why)),
        }
    }

    pub fn create_notification(&mut self, node_id: NodeId, content: &str) -> Result<(), Error> {
        self.0.execute(
            include_str!("../../queries/create_notification.sql"),
            &[&node_id, &content],
        )?;

        Ok(())
    }

    pub fn get_settings(&mut self, node_id: NodeId) -> Result<Option<NodeSettings>, Error> {
        let result = self.0.query_one(
            include_str!("../../queries/get_device_settings.sql"),
            &[&node_id],
        );

        match result {
            Ok(record) => Ok(Some(NodeSettings {
                battery_ignore: record.try_get("battery_ignore")?,
                ota: record.try_get("ota")?,
                sleep_time: record.get::<_, i16>("sleep_time").try_into()?,
                sbop: record.try_get("sbop")?,
                mute_notifications: record.try_get("mute_notifications")?,
            })),
            Err(e) if e.code() == Some(&SqlState::TOO_MANY_ROWS) => Ok(None),
            Err(why) => Err(why.into()),
        }
    }

    pub fn post_results(
        &mut self,
        node: NodeId,
        temp: Temperature,
        hum: Humidity,
        air_p: Option<AirPressure>,
    ) -> Result<MeasurementId, Error> {
        let signed_air_p: Option<i16> = match air_p {
            Some(value) => Some(value.try_into()?),
            None => None,
        };

        let result = self.0.query_one(
            include_str!("../../queries/post_results.sql"),
            &[
                &node,
                &temp.to_string(),
                &hum.to_string(),
                &i16::from(hum),
                &signed_air_p,
            ],
        )?;

        Ok(result.try_get("id")?)
    }

    pub fn post_stats(
        &mut self,
        measurement: MeasurementId,
        battery: &BatteryVoltage,
        wifi_ssid: &str,
        wifi_rssi: Rssi,
    ) -> Result<(), Error> {
        self.0.execute(
            include_str!("../../queries/post_stats.sql"),
            &[
                &measurement,
                &battery.to_string(),
                &wifi_ssid,
                &i16::from(wifi_rssi),
            ],
        )?;

        Ok(())
    }

    pub fn run_migrations(&mut self) -> Result<(), Error> {
        const QUERIES: [&str; 7] = [
            include_str!("../../migrations/20241217192210_create_devices_table.sql"),
            include_str!("../../migrations/20241217193005_create_measurements_table.sql"),
            include_str!("../../migrations/20241217193040_create_statistics_table.sql"),
            include_str!("../../migrations/20241217193101_create_settings_table.sql"),
            include_str!("../../migrations/20241217193123_create_notifications_table.sql"),
            include_str!("../../migrations/20241217193140_create_firmwares_table.sql"),
            include_str!("../../migrations/20241217193159_create_firmware_stats_table.sql"),
        ];

        for query in QUERIES {
            self.0.execute(query, &[])?;
        }

        Ok(())
    }

    pub fn check_os_update(
        &mut self,
        node: NodeId,
        current_ver: Version,
    ) -> Result<Option<(Version, FirmwareBlob)>, Error> {
        let (version_major, version_middle, version_minor) = current_ver.to_signed_triple();
        let result = self.0.query_one(
            include_str!("../../queries/get_os_update.sql"),
            &[&node, &version_major, &version_middle, &version_minor],
        );

        match result {
            Ok(row) => {
                let version = Version::new(
                    row.get::<_, i8>("version_major").try_into()?,
                    row.get::<_, i8>("version_middle").try_into()?,
                    row.get::<_, i8>("version_minor").try_into()?,
                );

                Ok(Some((
                    version,
                    row.get::<_, Vec<u8>>("firmware").into_boxed_slice(),
                )))
            }
            Err(e) if e.code() == Some(&SqlState::TOO_MANY_ROWS) => Ok(None),
            Err(why) => Err(Error::Database(why)),
        }
    }

    pub fn send_os_update_stat(
        &mut self,
        node_id: NodeId,
        old_ver: Version,
        new_ver: Version,
    ) -> Result<UpdateStatId, Error> {
        let (old_version_major, old_version_middle, old_version_minor) = old_ver.to_signed_triple();
        let (new_version_major, new_version_middle, new_version_minor) = new_ver.to_signed_triple();

        let result = self.0.query_one(
            include_str!("../../queries/send_os_update_event.sql"),
            &[
                &node_id,
                &old_version_major,
                &old_version_middle,
                &old_version_minor,
                &new_version_major,
                &new_version_middle,
                &new_version_minor,
            ],
        )?;

        Ok(result.try_get("id")?)
    }

    pub fn mark_os_update_stat(&mut self, node_id: NodeId, success: bool) -> Result<(), Error> {
        let last_update_id = self.get_last_os_update_stat(node_id)?;

        self.0.execute(
            include_str!("../../queries/update_os_update_event.sql"),
            &[&last_update_id, &success],
        )?;

        Ok(())
    }

    pub fn erase(&mut self, content_only: bool, keep_devices: bool) -> Result<(), Error> {
        let query: &str;
        let mut drop_migrations = false;

        if content_only {
            if keep_devices {
                query = include_str!("../../queries/erase_database_content_keep_devices.sql");
            } else {
                query = include_str!("../../queries/erase_database_content.sql");
            }

            drop_migrations = true;
        } else {
            query = include_str!("../../queries/erase_database.sql");
        }

        self.0.execute(query, &[])?;
        if drop_migrations {
            self.0
                .execute(include_str!("../../queries/drop_migrations_table.sql"), &[])?;
        }

        Ok(())
    }

    fn get_last_os_update_stat(&mut self, node_id: NodeId) -> Result<UpdateStatId, Error> {
        let row = self.0.query_one(
            include_str!("../../queries/get_last_update_event.sql"),
            &[&node_id],
        )?;

        Ok(row.try_get::<_, UpdateStatId>("id")?)
    }
}

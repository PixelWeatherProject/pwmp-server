use super::Backend;
use crate::server::db::PoolTrait;
use pwmp_client::pwmp_msg::version::Version;
use r2d2_postgres::postgres::NoTls;

pub struct PostgresPool(r2d2_postgres::PostgresConnectionManager<NoTls>);
pub struct Postgres(r2d2_postgres::postgres::Client);

impl PoolTrait for PostgresPool {
    fn connect(_config: &crate::server::config::Config) -> Result<Self, crate::error::Error>
    where
        Self: std::marker::Sized,
    {
        todo!()
    }

    fn get(&self) -> Result<crate::server::db::DatabaseClient, crate::error::Error> {
        todo!()
    }
}

impl Backend for Postgres {
    fn authorize_device(
        &self,
        mac: &pwmp_client::pwmp_msg::mac::Mac,
    ) -> Result<Option<crate::server::db::NodeId>, crate::error::Error> {
        todo!()
    }

    fn check_os_update(
        &self,
        node: crate::server::db::NodeId,
        current_ver: pwmp_client::pwmp_msg::version::Version,
    ) -> Result<
        Option<(
            pwmp_client::pwmp_msg::version::Version,
            crate::server::db::FirmwareBlob,
        )>,
        crate::error::Error,
    > {
        todo!()
    }

    fn create_notification(
        &self,
        node_id: crate::server::db::NodeId,
        content: &str,
    ) -> Result<(), crate::error::Error> {
        todo!()
    }

    fn erase(&self, content_only: bool, keep_devices: bool) -> Result<(), crate::error::Error> {
        todo!()
    }

    fn get_settings(
        &self,
        node_id: crate::server::db::NodeId,
    ) -> Result<Option<pwmp_client::pwmp_msg::settings::NodeSettings>, crate::error::Error> {
        todo!()
    }

    fn mark_os_update_stat(
        &self,
        node_id: crate::server::db::NodeId,
        success: bool,
    ) -> Result<(), crate::error::Error> {
        todo!()
    }

    fn post_results(
        &self,
        node: crate::server::db::NodeId,
        temp: pwmp_client::pwmp_msg::aliases::Temperature,
        hum: pwmp_client::pwmp_msg::aliases::Humidity,
        air_p: Option<pwmp_client::pwmp_msg::aliases::AirPressure>,
    ) -> Result<crate::server::db::MeasurementId, crate::error::Error> {
        todo!()
    }

    fn post_stats(
        &self,
        measurement: crate::server::db::MeasurementId,
        battery: &pwmp_client::pwmp_msg::aliases::BatteryVoltage,
        wifi_ssid: &str,
        wifi_rssi: pwmp_client::pwmp_msg::aliases::Rssi,
    ) -> Result<(), crate::error::Error> {
        todo!()
    }

    fn run_migrations(&self) -> Result<(), crate::error::Error> {
        todo!()
    }

    fn send_os_update_stat(
        &self,
        node_id: crate::server::db::NodeId,
        old_ver: pwmp_client::pwmp_msg::version::Version,
        new_ver: Version,
    ) -> Result<crate::server::db::UpdateStatId, crate::error::Error> {
        todo!()
    }
}

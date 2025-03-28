use super::{FirmwareBlob, MeasurementId, NodeId, UpdateStatId};
use crate::error::Error;
use pwmp_client::pwmp_msg::{
    aliases::{AirPressure, BatteryVoltage, Humidity, Rssi, Temperature},
    mac::Mac,
    settings::NodeSettings,
    version::Version,
};

pub mod postgres;
pub mod sqlite;

pub trait Backend: Send {
    fn authorize_device(&self, mac: &Mac) -> Result<Option<NodeId>, Error>;

    fn create_notification(&self, node_id: NodeId, content: &str) -> Result<(), Error>;

    fn get_settings(&self, node_id: NodeId) -> Result<Option<NodeSettings>, Error>;

    fn post_results(
        &self,
        node: NodeId,
        temp: Temperature,
        hum: Humidity,
        air_p: Option<AirPressure>,
    ) -> Result<MeasurementId, Error>;

    fn post_stats(
        &self,
        measurement: MeasurementId,
        battery: &BatteryVoltage,
        wifi_ssid: &str,
        wifi_rssi: Rssi,
    ) -> Result<(), Error>;

    fn run_migrations(&self) -> Result<(), Error>;

    fn check_os_update(
        &self,
        node: NodeId,
        current_ver: Version,
    ) -> Result<Option<(Version, FirmwareBlob)>, Error>;

    fn send_os_update_stat(
        &self,
        node_id: NodeId,
        old_ver: Version,
        new_ver: Version,
    ) -> Result<UpdateStatId, Error>;

    fn mark_os_update_stat(&self, node_id: NodeId, success: bool) -> Result<(), Error>;

    fn erase(&self, content_only: bool, keep_devices: bool) -> Result<(), Error>;
}

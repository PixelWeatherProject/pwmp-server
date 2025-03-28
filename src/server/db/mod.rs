use super::config::Config;
use crate::error::Error;
use backends::Backend;

pub type NodeId = i32;
pub type MeasurementId = i32;
pub type FirmwareBlob = Box<[u8]>;
pub type UpdateStatId = i32;

pub mod backends;

pub type DatabaseClient = Box<dyn Backend>;

pub struct Pool(Box<dyn PoolTrait>);

pub trait PoolTrait {
    fn connect(config: &Config) -> Result<Self, Error>
    where
        Self: std::marker::Sized;

    fn get(&self) -> Result<DatabaseClient, Error>;
}

impl PoolTrait for Pool {
    fn connect(_config: &Config) -> Result<Self, Error>
    where
        Self: std::marker::Sized,
    {
        todo!()
    }

    fn get(&self) -> Result<DatabaseClient, Error> {
        self.0.get()
    }
}

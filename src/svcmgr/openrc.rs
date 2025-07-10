use super::traits::ServiceManager;
use crate::error::Error;

pub struct Manager;

impl ServiceManager for Manager {
    fn detect(&self) -> bool {
        false
    }

    fn installed(&self) -> bool {
        unimplemented!()
    }

    fn running(&self) -> Result<bool, Error> {
        unimplemented!()
    }

    fn install(&self) -> Result<(), Error> {
        unimplemented!()
    }

    fn uninstall(&self) -> Result<(), Error> {
        unimplemented!()
    }

    fn disable(&self) -> Result<(), Error> {
        unimplemented!()
    }

    fn enable(&self) -> Result<(), Error> {
        unimplemented!()
    }

    fn enabled(&self) -> Result<bool, Error> {
        unimplemented!()
    }

    fn start(&self) -> Result<(), Error> {
        unimplemented!()
    }

    fn stop(&self) -> Result<(), Error> {
        unimplemented!()
    }
}

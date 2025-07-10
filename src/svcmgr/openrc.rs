use super::traits::ServiceManager;
use crate::error::Error;
use regex::Regex;
use std::process::Command;
use tracing::{error, info};

pub struct Manager;

impl ServiceManager for Manager {
    fn detect(&self) -> bool {
        let Ok(output) = super::exec_command(Command::new("openrc").arg("--version")) else {
            error!("Failed to execute OpenRC CLI");
            return false;
        };

        let version_regex = Regex::new(r#"\d+.\d+.\d+"#).unwrap();
        let Some(version_string) = version_regex.captures(&output).and_then(|res| res.get(0))
        else {
            error!("Failed to parse SystemD version string");
            return false;
        };

        info!("Found OpenRC v{}", version_string.as_str());
        true
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

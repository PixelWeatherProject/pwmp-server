use super::traits::ServiceManager;
use crate::error::Error;
use regex::Regex;
use std::{fs::OpenOptions, io::Write, path::PathBuf, process::Command};
use tracing::{error, info};

const SVCDIR: &str = "/etc/systemd/system";
const SVCNAME: &str = "pwmp-server";
const SVCEXT: &str = "service";
const CMDLINE_CLIENT: &str = "systemctl";

pub struct Manager;

impl Manager {
    fn service_file_path() -> PathBuf {
        let mut path = PathBuf::from(SVCDIR);
        path.push(SVCNAME);
        path.set_extension(SVCEXT);
        path
    }
}

impl ServiceManager for Manager {
    fn detect(&self) -> bool {
        let Ok(output) = super::exec_command(Command::new(CMDLINE_CLIENT).arg("--version")) else {
            error!("Failed to execute SystemD CLI");
            return false;
        };

        let version_regex = Regex::new(r"\((.*)\)").unwrap();
        let Some(version_string) = version_regex.captures(&output).and_then(|res| res.get(1))
        else {
            error!("Failed to parse SystemD version string");
            return false;
        };

        info!("Found SystemD v{}", version_string.as_str());
        true
    }

    fn installed(&self) -> bool {
        let mut svcfile = PathBuf::from(SVCDIR);
        svcfile.push(format!("{SVCNAME}.{SVCEXT}"));
        svcfile.is_file()
    }

    fn running(&self) -> Result<bool, Error> {
        let status =
            super::exec_command(Command::new(CMDLINE_CLIENT).args(["is-active", SVCNAME]))?;
        Ok(status == "active")
    }

    fn enabled(&self) -> Result<bool, Error> {
        let status =
            super::exec_command(Command::new(CMDLINE_CLIENT).args(["is-enabled", SVCNAME]))?;
        Ok(status == "enabled")
    }

    fn install(&self) -> Result<(), Error> {
        let mut svcfile = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(Self::service_file_path())?;

        let mut svc = include_str!("templates/systemd.service").to_string();
        svc = svc.replace("{user}", &whoami::username());
        svc = svc.replace(
            "{exec}",
            &std::env::current_exe().map(|path| path.display().to_string())?,
        );

        svcfile.write_all(svc.as_bytes())?;
        svcfile.flush()?;

        Ok(())
    }

    fn uninstall(&self) -> Result<(), Error> {
        Ok(std::fs::remove_file(Self::service_file_path())?)
    }

    fn enable(&self) -> Result<(), Error> {
        Command::new(CMDLINE_CLIENT)
            .args(["enable", SVCNAME])
            .output()?;
        Ok(())
    }

    fn disable(&self) -> Result<(), Error> {
        Command::new(CMDLINE_CLIENT)
            .args(["disable", SVCNAME])
            .output()?;
        Ok(())
    }

    fn start(&self) -> Result<(), Error> {
        Command::new(CMDLINE_CLIENT)
            .args(["start", SVCNAME])
            .output()?;
        Ok(())
    }

    fn stop(&self) -> Result<(), Error> {
        Command::new(CMDLINE_CLIENT)
            .args(["stop", SVCNAME])
            .output()?;
        Ok(())
    }
}

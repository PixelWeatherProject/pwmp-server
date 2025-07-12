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

    fn call_cli<I, S>(operation: &'static str, args: I) -> Result<std::process::Output, Error>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        let mut command = Command::new(CMDLINE_CLIENT);
        if !operation.is_empty() {
            command.arg(operation);
        }
        command.args(args);

        let output = command.output()?;
        if !output.status.success() {
            return Err(Error::SubprocessExit);
        }

        Ok(output)
    }

    fn simple_call_cli<I, S>(operation: &'static str, args: I) -> Result<(), Error>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        Self::call_cli(operation, args)?;
        Ok(())
    }

    fn call_cli_get_output<I, S>(operation: &'static str, args: I) -> Result<String, Error>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        let output = Self::call_cli(operation, args)?;
        Ok(String::from_utf8(output.stdout)?)
    }
}

impl ServiceManager for Manager {
    fn detect(&self) -> bool {
        let Ok(output) = Self::call_cli_get_output("", ["--version"]) else {
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
        Self::service_file_path().is_file()
    }

    fn running(&self) -> Result<bool, Error> {
        Ok(Self::call_cli_get_output("is-active", [SVCNAME])? == "active")
    }

    fn enabled(&self) -> Result<bool, Error> {
        Ok(Self::call_cli_get_output("is-enabled", [SVCNAME])? == "enabled")
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
        Self::simple_call_cli("enable", [SVCNAME])
    }

    fn disable(&self) -> Result<(), Error> {
        Self::simple_call_cli("disable", [SVCNAME])
    }

    fn start(&self) -> Result<(), Error> {
        Self::simple_call_cli("start", [SVCNAME])
    }

    fn stop(&self) -> Result<(), Error> {
        Self::simple_call_cli("stop", [SVCNAME])
    }
}

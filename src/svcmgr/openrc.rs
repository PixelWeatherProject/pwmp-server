use super::traits::ServiceManager;
use crate::error::Error;
use regex::Regex;
use std::{
    fs::OpenOptions, io::Write, os::unix::fs::PermissionsExt, path::PathBuf, process::Command,
};
use tracing::{error, info};

const SVCDIR: &str = "/etc/init.d";
const SVCNAME: &str = "pwmp-server";

#[derive(Clone, Copy)]
enum CliCmd {
    RcService,
    RcUpdate,
}

pub struct Manager;

impl Manager {
    fn service_file_path() -> PathBuf {
        let mut path = PathBuf::from(SVCDIR);
        path.push(SVCNAME);
        path
    }

    fn call_cli<I, S>(cmd: CliCmd, args: I) -> Result<std::process::Output, Error>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        let output = Command::new(cmd.as_ref()).args(args).output()?;

        // OpenRC's `rc-service <svc> status` command returns different exit codes depending on the status of
        // the service (eg. 3 if it's stopped), so we can't just check if the exit code is 0.
        if !output.status.success() && output.status.code() != Some(3) {
            return Err(Error::SubprocessExit);
        }

        Ok(output)
    }

    fn simple_call_cli<I, S>(cmd: CliCmd, args: I) -> Result<(), Error>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        Self::call_cli(cmd, args)?;
        Ok(())
    }

    fn call_cli_get_output<I, S>(cmd: CliCmd, args: I) -> Result<String, Error>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        let output = Self::call_cli(cmd, args)?;
        let output_as_str = String::from_utf8(output.stdout)?;
        Ok(output_as_str.trim_end().to_string()) /* remove any newlines at the end */
    }
}

impl ServiceManager for Manager {
    fn detect(&self) -> bool {
        let Ok(output) = Self::call_cli_get_output(CliCmd::RcService, ["--version"]) else {
            error!("Failed to execute OpenRC CLI");
            return false;
        };

        let version_regex = Regex::new(r"\d+.\d+.\d+").unwrap();
        let Some(version_string) = version_regex.captures(&output).and_then(|res| res.get(0))
        else {
            error!("Failed to parse OpenRC version string");
            return false;
        };

        info!("Found OpenRC v{}", version_string.as_str());
        true
    }

    fn installed(&self) -> bool {
        Self::service_file_path().is_file()
    }

    fn running(&self) -> Result<bool, Error> {
        Ok(
            Self::call_cli_get_output(CliCmd::RcService, [SVCNAME, "status"])?
                == " * status: started",
        )
    }

    fn enabled(&self) -> Result<bool, Error> {
        let regex = Regex::new(r"pwmp-server \|.*default.*").unwrap();
        let services = Self::call_cli_get_output(CliCmd::RcUpdate, ["show"])?;
        Ok(regex.is_match(&services))
    }

    fn install(&self) -> Result<(), Error> {
        // Generate the path
        let svcfile_path = Self::service_file_path();

        // Create the service file in a scope so it's closed at the end.
        {
            let mut svcfile = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&svcfile_path)?;

            let mut svc = include_str!("templates/openrc").to_string();
            svc = svc.replace("{user}", &whoami::username());
            svc = svc.replace(
                "{exec}",
                &std::env::current_exe().map(|path| path.display().to_string())?,
            );

            svcfile.write_all(svc.as_bytes())?;
            svcfile.flush()?;
        }

        // Make the file executable
        let mut permissions = std::fs::metadata(&svcfile_path)?.permissions();
        let mode = permissions.mode();
        let new_mode = mode | 0o111; // +x for user, group, others - same as `chmod +x ...`

        permissions.set_mode(new_mode);
        std::fs::set_permissions(svcfile_path, permissions)?;

        Ok(())
    }

    fn uninstall(&self) -> Result<(), Error> {
        Ok(std::fs::remove_file(Self::service_file_path())?)
    }

    fn disable(&self) -> Result<(), Error> {
        Self::simple_call_cli(CliCmd::RcUpdate, ["del", SVCNAME, "default"])
    }

    fn enable(&self) -> Result<(), Error> {
        Self::simple_call_cli(CliCmd::RcUpdate, ["add", SVCNAME, "default"])
    }

    fn start(&self) -> Result<(), Error> {
        Self::simple_call_cli(CliCmd::RcService, [SVCNAME, "start"])
    }

    fn stop(&self) -> Result<(), Error> {
        Self::simple_call_cli(CliCmd::RcService, [SVCNAME, "stop"])
    }
}

impl AsRef<str> for CliCmd {
    fn as_ref(&self) -> &str {
        match self {
            Self::RcService => "rc-service",
            Self::RcUpdate => "rc-update",
        }
    }
}

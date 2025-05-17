use super::traits::ServiceManager;
use tracing::{debug, error, info};
use std::{
    env::current_exe,
    fs,
    io::{Error, Result},
    path::PathBuf,
    process::{Command, Output},
};
use users::{get_current_uid, get_user_by_uid};

const CMDLINE: &str = "systemd";
const SVCDIR: &str = "/etc/systemd/system";
const SVCNAME: &str = "pwmp-server";
const SVCEXT: &str = "service";
const CMDLINE_CLIENT: &str = "systemctl";

pub struct Manager;

impl Manager {
    pub fn full_service_path() -> PathBuf {
        let mut buf = PathBuf::from(SVCDIR);

        buf.push(format!("{SVCNAME}.{SVCEXT}"));
        buf
    }

    pub fn exec_client_cmd(
        &self,
        cmd: &str,
        check_installed: bool,
        check_exit_code: bool,
    ) -> Result<Output> {
        if check_installed && !self.installed()? {
            return Err(Error::other("Service is not installed"));
        }

        let out = Command::new(CMDLINE_CLIENT).args([cmd, SVCNAME]).output()?;

        if check_exit_code && !out.status.success() {
            return Err(Error::other(format!(
                "\"{CMDLINE_CLIENT}\" exited with status code {}",
                out.status.code().unwrap()
            )));
        }

        Ok(out)
    }
}

impl ServiceManager for Manager {
    fn detect(&self) -> bool {
        debug!("Detecting SystemD");

        let Ok(out) = Command::new(CMDLINE).arg("--version").output() else {
            debug!("Failed to execute \"{CMDLINE}\"");
            return false;
        };

        if !out.status.success() {
            error!(
                "\"{CMDLINE}\" exited with status code {}",
                out.status.clone()
            );
            return false;
        }

        debug!("Detected SystemD CLI");

        let Ok(text) = String::from_utf8(out.stdout) else {
            error!("Failed to parse output of \"{CMDLINE}\"");
            return false;
        };
        let ver_info = text.lines().next().unwrap();
        info!("Detected {ver_info}");

        if PathBuf::from(SVCDIR).read_dir().is_err() {
            error!("Could not check directory \"{SVCDIR}\"");
            return false;
        }

        true
    }

    fn installed(&self) -> Result<bool> {
        let svcfile = Self::full_service_path();

        Ok(svcfile.exists())
    }

    fn install(&self) -> Result<()> {
        let user = get_user_by_uid(get_current_uid()).unwrap();
        let this_exec = current_exe().unwrap();

        let svcfile_path = Self::full_service_path();
        let svcfile = format!(
            include_str!("templates/systemd.service"),
            user = user.name().to_string_lossy(),
            exec = this_exec.display()
        );

        info!(
            "Creating service \"{SVCNAME}.{SVCEXT}\" at \"{}\"",
            svcfile_path.display()
        );

        fs::write(svcfile_path, svcfile)?;
        Ok(())
    }

    fn uninstall(&self) -> Result<()> {
        if !self.installed()? {
            return Err(Error::other("Service is not installed"));
        }
        self.disable()?;

        fs::remove_file(Self::full_service_path())
    }

    fn enabled(&self) -> Result<bool> {
        let out = self.exec_client_cmd("is-enabled", true, true)?;
        let enabled = out.stdout == b"enabled\n";

        Ok(enabled)
    }

    fn enable(&self) -> Result<()> {
        self.exec_client_cmd("enable", true, true)?;
        Ok(())
    }

    fn disable(&self) -> Result<()> {
        self.exec_client_cmd("disable", true, true)?;
        Ok(())
    }

    fn start(&self) -> Result<()> {
        self.exec_client_cmd("start", true, true)?;
        Ok(())
    }

    fn stop(&self) -> Result<()> {
        self.exec_client_cmd("stop", true, true)?;
        Ok(())
    }

    fn running(&self) -> Result<bool> {
        let out = self.exec_client_cmd("is-active", true, true)?;
        let enabled = out.stdout == b"active\n";

        Ok(enabled)
    }
}

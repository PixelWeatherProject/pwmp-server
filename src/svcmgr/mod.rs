use self::traits::ServiceManager;
use crate::cli::ServiceCommand;
use log::{error, info, warn};
use std::{io, process::exit};

mod openrc;
mod systemd;
mod traits;

pub fn main(cmd: ServiceCommand) {
    let manager = detect_manager();

    match cmd {
        ServiceCommand::Status => {
            match (manager.installed(), manager.enabled(), manager.running()) {
                (Err(why), ..) => info!("Could not determine if the service is installed: {why}"),
                (_, Err(why), ..) => info!("Could not determine of the service is enabled: {why}"),
                (_, _, Err(why)) => info!("Could not determine if the service is running: {why}"),
                (Ok(false), ..) => {
                    info!("Service is not installed.");
                }
                (Ok(true), Ok(true), Ok(true)) => {
                    info!("Service is installed, enabled and running");
                }
                (Ok(true), Ok(true), Ok(false)) => {
                    info!("Service is installed and enabled but not running");
                }
                (Ok(true), Ok(false), Ok(false)) => {
                    info!("Service is installed, but is disabled and inactive");
                }
                (Ok(true), Ok(false), Ok(true)) => {
                    info!("Service is installed and running, but won't auto-start on boot");
                }
            }
        }
        ServiceCommand::Install => {
            perform_cmd(|| manager.install(), "install", "installed");
            warn!("The service must be enabled manually");
        }
        ServiceCommand::Uninstall => {
            perform_cmd(|| manager.uninstall(), "uninstall", "uninstalled");
        }
        ServiceCommand::Enable => {
            perform_cmd(|| manager.enable(), "enable", "enabled");
        }
        ServiceCommand::Disable => {
            perform_cmd(|| manager.disable(), "disable", "disabled");
        }
        ServiceCommand::Start => {
            perform_cmd(|| manager.start(), "start", "started");
        }
        ServiceCommand::Stop => {
            perform_cmd(|| manager.stop(), "stop", "stopped");
        }
        ServiceCommand::Reinstall => {
            main(ServiceCommand::Reinstall);
        }
    }
}

fn perform_cmd<F: FnOnce() -> io::Result<()>>(func: F, action_name: &str, action_past: &str) {
    if let Err(why) = func() {
        error!("Failed to {action_name} service: {why}");
        exit(1);
    }

    info!("Service {action_past} successfully");
}

fn detect_manager() -> Box<dyn ServiceManager> {
    if systemd::Manager.detect() {
        return Box::new(systemd::Manager);
    } else if openrc::Manager.detect() {
        return Box::new(openrc::Manager);
    }

    error!("Could not find a service manager on this system");
    exit(1);
}

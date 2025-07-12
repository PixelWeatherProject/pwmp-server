use self::traits::ServiceManager;
use crate::cli::ServiceCommand;
use std::process::exit;
use tracing::{debug, error, info, warn};

mod openrc;
mod systemd;
mod traits;

#[allow(clippy::too_many_lines, clippy::cognitive_complexity)]
pub fn main(cmd: ServiceCommand) {
    let manager = detect_manager();

    match cmd {
        ServiceCommand::Status => {
            if !manager.installed() {
                error!("Service is not installed");
                return;
            }

            let running = match manager.running() {
                Ok(res) => res,
                Err(why) => {
                    error!("Failed to check if the service is running: {why}");
                    exit(1);
                }
            };

            let enabled = match manager.enabled() {
                Ok(res) => res,
                Err(why) => {
                    error!("Failed to check if the service is enabled: {why}");
                    exit(1);
                }
            };

            info!("Running: {}", running);
            info!("Enabled: {}", enabled);
        }
        ServiceCommand::Install => {
            if manager.installed() {
                warn!("Service is already installed");
                return;
            }

            match manager.install() {
                Ok(()) => {
                    info!("Service has been installed successfully");
                    warn!("The service must be enabled and started manually");
                }
                Err(why) => {
                    error!("Failed to install the service: {why}");
                    exit(1);
                }
            }
        }
        ServiceCommand::Uninstall => {
            if !manager.installed() {
                error!("Service is not installed");
                exit(1);
            }

            debug!("Service is not running, skipping stop operation");
            if manager.running().is_ok_and(|r| r) {
                info!("Stopping the service");
                if let Err(why) = manager.stop() {
                    error!("Failed to stop the service: {why}");
                    exit(1);
                }
            }

            debug!("Service is not enabled, skipping disable operation");
            if manager.enabled().is_ok_and(|e| e) {
                info!("Disabling the service");
                if let Err(why) = manager.disable() {
                    error!("Failed to disable the service: {why}");
                    exit(1);
                }
            }

            match manager.uninstall() {
                Ok(()) => {
                    info!("Service has been uninstalled successfully");
                }
                Err(why) => {
                    error!("Failed to uninstall the service: {why}");
                    exit(1);
                }
            }
        }
        ServiceCommand::Enable => {
            if !manager.installed() {
                error!("Service is not installed");
                exit(1);
            }

            if manager.enabled().is_ok_and(|res| res) {
                warn!("Service is already enabled");
                return;
            }

            match manager.enable() {
                Ok(()) => {
                    info!("Service has been enabled successfully");
                }
                Err(why) => {
                    error!("Failed to enable the service: {why}");
                    exit(1);
                }
            }
        }
        ServiceCommand::Disable => {
            if !manager.installed() {
                error!("Service is not installed");
                exit(1);
            }

            if manager.enabled().is_ok_and(|res| !res) {
                warn!("Service is already disabled");
                return;
            }

            match manager.disable() {
                Ok(()) => {
                    info!("Service has been disabled successfully");
                }
                Err(why) => {
                    error!("Failed to disable the service: {why}");
                    exit(1);
                }
            }
        }
        ServiceCommand::Start => {
            if !manager.installed() {
                error!("Service is not installed");
                exit(1);
            }

            if manager.running().is_ok_and(|res| res) {
                warn!("Service is already running");
                return;
            }

            match manager.start() {
                Ok(()) => {
                    info!("Service has been started successfully");
                }
                Err(why) => {
                    error!("Failed to start the service: {why}");
                    exit(1);
                }
            }
        }
        ServiceCommand::Stop => match manager.stop() {
            Ok(()) => {
                info!("Service has been stopped successfully");
            }
            Err(why) => {
                error!("Failed to stop the service: {why}");
                exit(1);
            }
        },
        ServiceCommand::Reinstall => {
            if !manager.installed() {
                error!("Service is not installed");
                exit(1);
            }

            main(ServiceCommand::Uninstall);
            main(ServiceCommand::Install);
        }
    }
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

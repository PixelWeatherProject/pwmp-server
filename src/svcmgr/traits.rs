use crate::error::Error;

/// This contains methods that every service management software on *nix systems should be able to do.
pub trait ServiceManager {
    /// Detect if the system uses this service manager.
    fn detect(&self) -> bool;

    /// Check if the service is installed.
    fn installed(&self) -> bool;

    /// Check if the service is running.
    fn running(&self) -> Result<bool, Error>;

    /// Check if the service is enabled.
    fn enabled(&self) -> Result<bool, Error>;

    /// Install the service.
    fn install(&self) -> Result<(), Error>;

    /// Uninstall the service.
    fn uninstall(&self) -> Result<(), Error>;

    /// Enable the service.
    fn enable(&self) -> Result<(), Error>;

    /// Disable the service.
    fn disable(&self) -> Result<(), Error>;

    /// Start the service.
    fn start(&self) -> Result<(), Error>;

    /// Stop the service.
    fn stop(&self) -> Result<(), Error>;
}

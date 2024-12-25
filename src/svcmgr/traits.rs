use std::io::Result;

/// This contains methods that every service management software on *nix systems should be able to do.
pub trait ServiceManager {
    /// Detect if the system uses this service manager.
    fn detect(&self) -> bool;

    /// Check if the service is installed.
    fn installed(&self) -> Result<bool>;

    /// Check if the service is running.
    fn running(&self) -> Result<bool>;

    /// Install the service.
    fn install(&self) -> Result<()>;

    /// Uninstall the service.
    fn uninstall(&self) -> Result<()>;

    /// Check if the service is enabled.
    fn enabled(&self) -> Result<bool>;

    /// Enable the service.
    fn enable(&self) -> Result<()>;

    /// Disable the service.
    fn disable(&self) -> Result<()>;

    /// Start the service.
    fn start(&self) -> Result<()>;

    /// Stop the service.
    fn stop(&self) -> Result<()>;
}

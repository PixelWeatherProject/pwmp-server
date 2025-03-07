use clap::{command, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Alternative configuration file path
    #[arg(long, value_name = "PATH")]
    pub config: Option<PathBuf>,

    /// Enable debug logging
    #[arg(long)]
    pub debug: bool,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Service management
    Service {
        #[command(subcommand)]
        command: ServiceCommand,
    },

    /// Database management
    Database {
        #[command(subcommand)]
        command: DatabaseCommand,
    },

    /// Test connection to a PWMP server
    Test {
        /// Host to connect to
        host: String,
        /// MAC address to authenticate with
        mac: String,
        /// Alternative port to use
        port: Option<u16>,
    },
}

#[derive(Debug, Subcommand, Clone, Copy)]
pub enum ServiceCommand {
    /// Start the service
    Start,

    /// Stop the service
    Stop,

    /// Enable
    Enable,

    /// Disable
    Disable,

    /// Install as service
    Install,

    /// Uninstall service
    Uninstall,

    /// Check the status of the service
    Status,

    /// Reinstall service
    Reinstall,
}

#[derive(Debug, Subcommand, Clone, Copy)]
pub enum DatabaseCommand {
    /// Test connection to the database
    Test,

    /// Initialize the database
    Init,

    /// Completely ERASE ALL DATA from the database (*UNRECOVERABLE*)
    Erase {
        /// Only remove rows, not tables
        #[arg(long)]
        content_only: bool,

        /// Keep configured devices
        #[arg(long)]
        keep_devices: bool,
    },
}

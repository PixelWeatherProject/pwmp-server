use crate::server::db::NodeId;
use clap::{Parser, Subcommand};
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

    Ota {
        #[command(subcommand)]
        command: OtaCommand,
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

#[derive(Debug, Subcommand, Clone)]
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

#[derive(Debug, Subcommand, Clone)]
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

        /// Keep configured devices and their settings
        #[arg(long)]
        keep_devices: bool,
    },
}

#[derive(Debug, Subcommand, Clone)]
pub enum OtaCommand {
    /// List firmwares in the database
    List,

    /// Download a firmware binary from the database
    Pull { id: i32, output: PathBuf },

    /// Upload a firmware binary to the database
    Push {
        /// Path to the binary blob
        blob: PathBuf,

        /// Semantic version string
        version: String,

        /// Restrict the firmware to only be available for the specified node
        #[arg(short, long, action = clap::ArgAction::Append)]
        restrict: Vec<NodeId>,
    },
}

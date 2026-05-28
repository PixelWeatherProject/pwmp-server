#![allow(clippy::module_name_repetitions)]

use crate::error::Error;
use serde::{Deserialize, Serialize};
use serde_with::{DurationSeconds, serde_as};
use std::{
    net::{Ipv4Addr, SocketAddrV4},
    path::PathBuf,
    time::Duration,
};

#[serde_as]
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub cache: CacheConfig,
    pub limits: LimitsConfig,
    #[serde(rename = "rate_limiter")]
    pub rate_limits: RateLimitConfig,
    pub logging: LogConfig,
    pub notification: NotificationConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: Ipv4Addr,
    pub port: u16,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum DatabaseConfig {
    Postgres {
        host: Box<str>,
        port: u16,
        user: Box<str>,
        password: Box<str>,
        name: Box<str>,
        ssl: bool,
    },
    Sqlite {
        file: PathBuf,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CacheConfig {
    pub auth_ttl: Duration,
    pub auth_capacity: u64,

    pub settings_ttl: Duration,
    pub settings_capacity: u64,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct LimitsConfig {
    pub devices: u32,
    pub settings: u32,
    #[serde_as(as = "DurationSeconds")]
    pub stall_time: Duration,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub max_requests: usize,
    pub max_connections: usize,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct LogConfig {
    pub file: Option<PathBuf>,
    pub erase_file_on_start: bool,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct NotificationConfig {
    pub push_backend: Option<NotificationServiceConfig>,
    pub events: NotificationEventsConfig,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[allow(clippy::struct_excessive_bools)] // this is not a state machine
pub struct NotificationEventsConfig {
    pub on_update_discovered: bool,
    pub on_update_success: bool,
    pub on_update_failed: bool,
    pub on_measurements_posted: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum NotificationServiceConfig {
    Pushsafer {
        private_key: Box<str>,
        device: Box<str>,
    },
    HassNotify {
        url: Box<str>,
        token: Box<str>,
        target: Box<str>,
    },
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: 20,
            max_connections: 4,
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: Ipv4Addr::UNSPECIFIED,
            port: 55300,
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self::Postgres {
            host: "192.168.0.12".into(),
            port: 5432,
            user: "root".into(),
            password: "root".into(),
            name: "pixelweather".into(),
            ssl: false,
        }
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            auth_ttl: Duration::from_hours(1),
            auth_capacity: 10,
            settings_ttl: Duration::from_hours(1),
            settings_capacity: 10,
        }
    }
}

impl Default for LimitsConfig {
    fn default() -> Self {
        Self {
            devices: 10,
            settings: 10,
            stall_time: Duration::from_secs(10),
        }
    }
}

impl DatabaseConfig {
    pub fn host(&self) -> String {
        match self {
            Self::Postgres { host, .. } => host.to_string(),
            Self::Sqlite { file } => file.display().to_string(),
        }
    }

    pub fn name(&self) -> String {
        match self {
            Self::Postgres { name, .. } => name.to_string(),
            Self::Sqlite { file } => file
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
        }
    }
}

impl Config {
    pub fn default_path() -> PathBuf {
        homedir::my_home()
            .unwrap()
            .unwrap()
            .join(".pwmp-server/config.yml")
    }

    pub const fn server_bind_addr(&self) -> SocketAddrV4 {
        SocketAddrV4::new(self.server.host, self.server.port)
    }
}

pub fn setup(config_path: &PathBuf) -> Result<(Config, bool), Error> {
    let first_run = !config_path.exists();
    let config: Config = confy::load_path(config_path)?;

    Ok((config, first_run))
}

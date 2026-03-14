#![allow(clippy::module_name_repetitions)]

use serde::{Deserialize, Serialize};
use serde_with::{DurationSeconds, serde_as};
use std::{
    net::{Ipv4Addr, SocketAddrV4},
    path::PathBuf,
    time::Duration,
};

use crate::error::Error;

#[serde_as]
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub limits: LimitsConfig,
    #[serde(rename = "rate_limiter")]
    pub rate_limits: RateLimitConfig,
    pub logging: LogConfig,
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
        timezone: Option<String>,
    },
    Sqlite {
        file: PathBuf,
    },
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
    pub time_frame: u64,
    pub max_requests: usize,
    pub max_connections: usize,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct LogConfig {
    pub file: Option<PathBuf>,
    pub erase_file_on_start: bool,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            time_frame: 1,
            max_requests: 4,
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
            timezone: None,
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

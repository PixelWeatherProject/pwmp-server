#![allow(clippy::module_name_repetitions)]

use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DurationSeconds};
use std::{
    net::{Ipv4Addr, SocketAddrV4},
    path::PathBuf,
    time::Duration,
};

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub limits: LimitsConfig,
    #[serde(rename = "rate_limiter")]
    pub rate_limits: RateLimitConfig,
    #[serde_as(as = "DurationSeconds")]
    pub max_stall_time: Duration,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: Ipv4Addr,
    pub port: u16,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub host: Box<str>,
    pub port: u16,
    pub user: Box<str>,
    pub password: Box<str>,
    pub name: Box<str>,
    pub ssl: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LimitsConfig {
    pub max_devices: u32,
    pub max_settings: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub time_frame: u64,
    pub max_requests: usize,
    pub max_connections: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: Default::default(),
            database: Default::default(),
            limits: Default::default(),
            rate_limits: Default::default(),
            max_stall_time: Duration::from_secs(10),
        }
    }
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
            host: Ipv4Addr::new(0, 0, 0, 0),
            port: 55300,
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            host: "192.168.0.12".into(),
            port: 5432,
            user: "root".into(),
            password: "root".into(),
            name: "pixelweather".into(),
            ssl: false,
        }
    }
}

impl Default for LimitsConfig {
    fn default() -> Self {
        Self {
            max_devices: 10,
            max_settings: 10,
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

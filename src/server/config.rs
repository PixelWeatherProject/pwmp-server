#![allow(clippy::module_name_repetitions)]

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
    pub limits: LimitsConfig,
    #[serde(rename = "rate_limiter")]
    pub rate_limits: RateLimitConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: Ipv4Addr,
    pub port: u16,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum DatabaseConfig {
    Postgres(PostgresConfig),
    Sqlite(SqliteConfig),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PostgresConfig {
    pub host: Box<str>,
    pub port: u16,
    pub user: Box<str>,
    pub password: Box<str>,
    pub name: Box<str>,
    pub ssl: bool,
    pub timezone: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SqliteConfig {
    pub path: PathBuf,
    pub timezone: Option<String>,
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
        Self::Postgres(PostgresConfig::default())
    }
}

impl Default for PostgresConfig {
    fn default() -> Self {
        Self {
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

impl Default for SqliteConfig {
    fn default() -> Self {
        Self {
            path: PathBuf::from("/tmp/pixelweather.sqlite3"),
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

    pub fn short_db_identifier(&self) -> String {
        match &self.database {
            DatabaseConfig::Postgres(config) => config.host.to_string(),
            DatabaseConfig::Sqlite(config) => config.path.display().to_string(),
        }
    }

    pub fn db_timezone(&self) -> Option<String> {
        match &self.database {
            DatabaseConfig::Postgres(config) => config.timezone.clone(),
            DatabaseConfig::Sqlite(config) => config.timezone.clone(),
        }
        .or_else(|| iana_time_zone::get_timezone().ok())
    }
}

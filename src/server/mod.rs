use crate::server::{db::DatabaseClient, handle::server_loop};
use config::Config;
use log::{error, info};
use std::{net::TcpListener, process::exit, sync::Arc};

mod client;
mod client_handle;
pub mod config;
pub mod db;
pub mod handle;
pub mod rate_limit;

pub fn main(config: Config) {
    info!("Connecting to database at \"{}\"", config.database.host);
    let db = match DatabaseClient::new(&config) {
        Ok(db) => db,
        Err(why) => {
            error!("Failed to connect to database: {why}");
            exit(1);
        }
    };

    let Ok(server) = TcpListener::bind(config.server_bind_addr()) else {
        eprintln!("Failed to bind to {}", config.server_bind_addr());
        exit(1);
    };

    info!("Server started on {}", config.server_bind_addr());

    server_loop(&server, db, Arc::new(config));
}

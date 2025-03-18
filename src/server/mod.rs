use crate::server::{db::DatabaseClient, handle::server_loop};
use config::Config;
use log::{error, info};
use socket2::{Domain, Protocol, Socket, Type};
use std::{io, process::exit, sync::Arc, time::Duration};

mod client;
mod client_handle;
pub mod config;
pub mod db;
pub mod handle;
pub mod rate_limit;

pub fn main(config: Config) {
    let config = Arc::new(config);

    info!("Connecting to database at \"{}\"", config.database.host);
    let db = match DatabaseClient::new(&config) {
        Ok(db) => db,
        Err(why) => {
            error!("Failed to connect to database: {why}");
            exit(1);
        }
    };

    let server = match Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP)) {
        Ok(socket) => socket,
        Err(why) => {
            error!("Failed to create socket: {why}");
            exit(1);
        }
    };

    if let Err(why) = server.bind(&config.server_bind_addr().into()) {
        error!("Failed to bind to {}: {why}", config.server_bind_addr());
        exit(1);
    }

    if let Err(why) = set_global_socket_params(&server, &config) {
        error!("Failed to set up socket parameters: {why}");
        exit(1);
    }

    info!("Server started on {}", config.server_bind_addr());
    server_loop(&server, db, config);
}

pub fn set_global_socket_params(socket: &Socket, config: &Arc<Config>) -> io::Result<()> {
    socket.set_nodelay(true)?;
    socket.set_keepalive(true)?;
    socket.set_linger(Some(Duration::from_secs(1)))?;
    socket.set_read_timeout(Some(config.max_stall_time))?;
    socket.set_write_timeout(Some(config.max_stall_time))?;

    Ok(())
}

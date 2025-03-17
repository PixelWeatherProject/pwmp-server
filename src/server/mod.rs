use crate::server::{db::DatabaseClient, handle::server_loop};
use config::Config;
use log::{error, info};
use message_io::{network::Transport, node};
use std::{process::exit, sync::Arc};

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

    let (handler, listener) = node::split::<()>();

    match handler
        .network()
        .listen(Transport::FramedTcp, config.server_bind_addr())
    {
        Ok((_id, real_addr)) => info!("Server running at {}", real_addr),
        Err(why) => return error!("Failed to bind server socket: {why}"),
    }

    {
        let handle_copy = handler.clone();
        ctrlc::set_handler(move || handle_copy.stop()).unwrap();
    }

    info!("Server started on {}", config.server_bind_addr());
    server_loop(handler, listener, db, Arc::new(config));
}

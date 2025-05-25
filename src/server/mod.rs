use crate::server::{db::DatabaseClient, handle::server_loop};
use config::Config;
use socket2::SockRef;
use std::{io, os::fd::AsFd, process::exit, sync::Arc, time::Duration};
use tokio::{
    net::TcpListener,
    signal::unix::{Signal, SignalKind, signal},
};
use tracing::{debug, error, info, warn};

mod client;
mod client_handle;
pub mod config;
pub mod db;
pub mod handle;
pub mod rate_limit;

pub async fn main(config: Config) {
    let config = Arc::new(config);

    info!("Connecting to database at \"{}\"", config.database.host);
    let db = match DatabaseClient::new(&config).await {
        Ok(db) => db,
        Err(why) => {
            error!("Failed to connect to database: {why}");
            exit(1);
        }
    };

    debug!("Setting timezone");
    match config
        .database
        .timezone
        .clone()
        .or_else(|| iana_time_zone::get_timezone().ok())
    {
        Some(tz) => match db.setup_timezone(&tz).await {
            Ok(()) => info!("Timezone updated to \"{tz}\" successfully"),
            Err(why) => {
                error!("Failed to set time zone: {why}");
                warn!("Keeping timezone unset, timestamps may have unexpected values");
            }
        },
        None => {
            warn!("Cannot determine system time zone, skipping");
        }
    }

    let server = match TcpListener::bind(config.server_bind_addr()).await {
        Ok(socket) => socket,
        Err(why) => {
            error!("Failed to create socket: {why}");
            exit(1);
        }
    };

    if let Err(why) = set_global_socket_params(&server) {
        error!("Failed to set up socket parameters: {why}");
        exit(1);
    }

    let (stop_sig, ping_sig) = setup_signals();

    info!("Server started on {}", config.server_bind_addr());
    server_loop(&server, db, config, stop_sig, ping_sig).await;
}

fn setup_signals() -> (Signal, Signal) {
    let stop_sig =
        signal(SignalKind::interrupt()).expect("Failed to set up signal handler for SIGINT");
    let ping_sig =
        signal(SignalKind::user_defined1()).expect("Failed to set up signal handler for SIGUSR1");

    (stop_sig, ping_sig)
}

pub fn set_global_socket_params<S: AsFd>(socket: &S) -> io::Result<()> {
    let socket: SockRef = socket.into();

    socket.set_nodelay(true)?;
    socket.set_keepalive(true)?;
    socket.set_linger(Some(Duration::from_secs(1)))?;

    Ok(())
}

use crate::server::{db::DatabaseClient, handle::server_loop};
use config::Config;
use libc::{IPPROTO_TCP, SO_KEEPALIVE, SO_LINGER, SOL_SOCKET, TCP_NODELAY, linger, socklen_t};
use log::{debug, error, info, warn};
use std::{ffi::c_int, io, mem, os::fd::AsRawFd, process::exit, sync::Arc};
use tokio::{
    net::TcpListener,
    signal::unix::{Signal, SignalKind, signal},
};

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

pub fn set_global_socket_params<FD: AsRawFd>(socket: &FD) -> io::Result<()> {
    setsockopt(
        socket,
        SOL_SOCKET,
        SO_LINGER,
        linger {
            l_linger: 5,
            l_onoff: 1,
        },
    )?;
    setsockopt(socket, IPPROTO_TCP, TCP_NODELAY, &1)?;
    setsockopt(socket, SOL_SOCKET, SO_KEEPALIVE, &1i32)?;

    Ok(())
}

#[allow(clippy::needless_pass_by_value)] // Passing a reference to T causes this to break. Possibly because `ptr` becomes a double pointer?
fn setsockopt<T, FD: AsRawFd>(fd: &FD, level: c_int, opt: c_int, value: T) -> io::Result<()> {
    let (ptr, len) = ((&raw const value).cast(), mem::size_of::<T>());
    let option_len = socklen_t::try_from(len)
        .map_err(|_| io::Error::other("failed to convert usize to socklen_t"))?;
    let err = unsafe { libc::setsockopt(fd.as_raw_fd(), level, opt, ptr, option_len) };

    if err == 0 {
        return Ok(());
    }

    Err(io::Error::last_os_error())
}

use crate::server::{db::DatabaseClient, handle::server_loop};
use config::Config;
use libc::{
    IPPROTO_TCP, SO_KEEPALIVE, SO_LINGER, SO_RCVTIMEO, SO_SNDTIMEO, SOL_SOCKET, TCP_NODELAY,
    linger, socklen_t, timeval,
};
use log::{error, info};
use signal::SignalHandle;
use std::{
    ffi::c_int, io, mem, net::TcpListener, os::fd::AsRawFd, process::exit, sync::Arc, thread,
};

mod client;
mod client_handle;
pub mod config;
pub mod db;
pub mod handle;
pub mod rate_limit;
pub mod signal;

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

    let server = match TcpListener::bind(config.server_bind_addr()) {
        Ok(socket) => socket,
        Err(why) => {
            error!("Failed to create socket: {why}");
            exit(1);
        }
    };

    if let Err(why) = set_global_socket_params(&server, &config) {
        error!("Failed to set up socket parameters: {why}");
        exit(1);
    }

    let (stop_sig, ping_sig) = setup_signals();

    info!("Server started on {}", config.server_bind_addr());
    server_loop(&server, db, config, stop_sig, ping_sig);
}

fn setup_signals() -> (SignalHandle, SignalHandle) {
    let stop_sig = SignalHandle::new(signal_hook::consts::SIGINT);
    let ping_sing = SignalHandle::new(signal_hook::consts::SIGUSR1);

    {
        let stop_sig_copy = stop_sig.clone();

        thread::spawn(move || {
            while !stop_sig_copy.is_set() {}
            info!("Stop requested, please wait");
        });
    }

    (stop_sig, ping_sing)
}

pub fn set_global_socket_params<FD: AsRawFd>(socket: &FD, config: &Arc<Config>) -> io::Result<()> {
    setsockopt(
        socket,
        SOL_SOCKET,
        SO_SNDTIMEO, /* write timeout */
        timeval {
            tv_sec: config.limits.stall_time.as_secs().try_into().unwrap(),
            tv_usec: 0,
        },
    )?;
    setsockopt(
        socket,
        SOL_SOCKET,
        SO_RCVTIMEO, /* read timeout */
        timeval {
            tv_sec: config.limits.stall_time.as_secs().try_into().unwrap(),
            tv_usec: 0,
        },
    )?;
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

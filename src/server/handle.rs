use super::{config::Config, db::DatabaseClient, rate_limit::RateLimiter};
use crate::server::client_handle::handle_client;
use log::{debug, error, warn};
use semaphore::Semaphore;
use std::{io::ErrorKind, net::TcpListener, panic, sync::Arc, thread, time::Duration};

#[allow(clippy::needless_pass_by_value)]
pub fn server_loop(server: &TcpListener, db: DatabaseClient, config: Arc<Config>) {
    let shared_db = Arc::new(db);
    let connections = Semaphore::new(config.limits.devices as _, ());
    let mut rate_limiter = RateLimiter::new(
        Duration::from_secs(config.rate_limits.time_frame),
        config.rate_limits.max_connections,
    );

    loop {
        let (client, peer_addr) = match server.accept() {
            Ok(res) => res,
            Err(why) if why.kind() == ErrorKind::WouldBlock => continue,
            Err(why) => {
                error!("Failed to accept connection: {why}");
                continue;
            }
        };

        debug!("New client: {peer_addr}");

        debug!("Incrementing connection count");
        let Ok(semguard) = connections.try_access() else {
            warn!("Maximum number of connections reached, ignoring connection");
            continue;
        };

        if rate_limiter.hit() {
            warn!("Exceeded rate limit for accepting incoming connections");
            continue;
        }

        debug!("{peer_addr:?}: Setting socket parameters");
        if let Err(why) = super::set_global_socket_params(&client, &config) {
            error!("{peer_addr:?}: Failed to set socket parameters: {why}");
            continue;
        };

        {
            let config = Arc::clone(&config);
            let db = shared_db.clone();

            debug!("Starting client thread");
            thread::spawn(move || {
                let _semguard = semguard;

                debug!("Setting panic hook for thread");
                set_panic_hook();

                debug!("Starting client handle");
                match handle_client(client, &db, config) {
                    Ok(()) => {
                        debug!("{peer_addr}: Handled successfully");
                    }
                    Err(why) => {
                        error!("{peer_addr}: Failed to handle: {why}");
                    }
                }
            });
        }
    }
}

fn set_panic_hook() {
    panic::set_hook(Box::new(move |info| {
        warn!("A client thread has paniced: {info:?}");
    }));
}

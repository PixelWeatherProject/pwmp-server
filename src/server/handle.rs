use super::{config::Config, db::DatabaseClient, rate_limit::RateLimiter, signal::SignalHandle};
use crate::server::client_handle::handle_client;
use log::{debug, error, info, warn};
use semaphore::Semaphore;
use std::{panic, sync::Arc, time::Duration};
use tokio::net::TcpListener;

#[allow(clippy::needless_pass_by_value)]
pub async fn server_loop(
    server: &TcpListener,
    db: DatabaseClient,
    config: Arc<Config>,
    stop_sig: SignalHandle,
    ping_sig: SignalHandle,
) {
    let shared_db = Arc::new(db);
    let connections = Semaphore::new(config.limits.devices as _, ());
    let mut rate_limiter = RateLimiter::new(
        Duration::from_secs(config.rate_limits.time_frame),
        config.rate_limits.max_connections,
    );

    loop {
        if stop_sig.is_set() {
            info!("Stopping server");
            break;
        }

        if ping_sig.is_set() {
            info!("Ping requested through SIGUSR1");
            ping_sig.unset();
        }

        let (client, peer_addr) = match server.accept().await {
            Ok(res) => res,
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
        if let Err(why) = super::set_global_socket_params(&client) {
            error!("{peer_addr:?}: Failed to set socket parameters: {why}");
            continue;
        }

        {
            let config = Arc::clone(&config);
            let db = shared_db.clone();

            debug!("Starting client task");
            tokio::spawn(async move {
                let _semguard = semguard;

                debug!("Setting panic hook for thread");
                set_panic_hook();

                debug!("Starting client handle");
                match handle_client(client, &db, config).await {
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

use super::{config::Config, db::DatabaseClient, rate_limit::RateLimiter};
use crate::server::client_handle::handle_client;
use log::{debug, error, warn};
use std::{
    net::TcpListener,
    panic,
    sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
    },
    thread,
    time::Duration,
};

static CONNECTIONS: AtomicU32 = AtomicU32::new(0);

#[allow(clippy::needless_pass_by_value)]
pub fn server_loop(server: &TcpListener, db: DatabaseClient, config: Arc<Config>) {
    let shared_db = Arc::new(db);
    let mut rate_limiter = RateLimiter::new(
        Duration::from_secs(config.rate_limits.time_frame),
        config.rate_limits.max_connections,
    );

    for client in server.incoming() {
        if CONNECTIONS.load(Ordering::Relaxed) == config.limits.max_devices {
            warn!("Maximum number of connections reached, ignoring connection");
            continue;
        }

        if rate_limiter.hit() {
            warn!("Exceeded rate limit for accepting incoming connections");
            continue;
        }

        let Ok(client) = client else {
            warn!("A client failed to connect");
            continue;
        };
        let Ok(peer_addr) = client.peer_addr() else {
            error!("Failed to get a client's peer address information");
            continue;
        };

        debug!("Incrementing connection count");
        CONNECTIONS.fetch_add(1, Ordering::Relaxed);
        if CONNECTIONS.load(Ordering::Relaxed) == config.limits.max_devices {
            warn!("Reached maximum number of connections, new connections will be blocked");
        }

        {
            let config = Arc::clone(&config);
            let db = shared_db.clone();
            debug!("Connection count: {}", CONNECTIONS.load(Ordering::SeqCst));

            debug!("Starting client thread");
            thread::spawn(move || {
                debug!("New client: {}", peer_addr);
                debug!("Setting panic hook for thread");
                set_panic_hook();

                debug!("Starting client handle");
                match handle_client(client, &db, config) {
                    Ok(()) => {
                        debug!("{}: Handled successfully", peer_addr);
                    }
                    Err(why) => {
                        error!("{peer_addr}: Failed to handle: {why}");
                    }
                }

                debug!("Decrementing connection count");
                CONNECTIONS.fetch_sub(1, Ordering::Relaxed);
            });
        }
    }
}

fn set_panic_hook() {
    let default = panic::take_hook();

    panic::set_hook(Box::new(move |panic_info| {
        warn!("A client thread has paniced");
        CONNECTIONS.fetch_sub(1, Ordering::Relaxed);
        default(panic_info);
    }));
}

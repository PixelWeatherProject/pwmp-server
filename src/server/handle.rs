use super::{config::Config, db::DatabaseClient, rate_limit::RateLimiter};
use crate::server::client_handle::handle_client;
use log::{debug, error, warn};
use std::{
    net::TcpListener,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

#[allow(clippy::needless_pass_by_value)]
pub fn server_loop(server: &TcpListener, db: DatabaseClient, config: Arc<Config>) {
    let connections = Arc::new(AtomicU32::new(0));
    let shared_db = Arc::new(db);
    let mut rate_limiter = RateLimiter::new(
        Duration::from_secs(config.rate_limits.time_frame),
        config.rate_limits.max_connections,
    );

    for client in server.incoming() {
        if connections.load(Ordering::Relaxed) == config.limits.max_devices {
            warn!("Maximum number of connections reached, ignoring connection");
            continue;
        }

        if rate_limiter.hit() {
            warn!("Rate limiting");
            continue;
        }

        let Ok(client) = client else {
            warn!("A client failed to connect");
            continue;
        };
        let Ok(peer_addr) = client.peer_addr() else {
            error!("Failed to get a clients peer address information");
            continue;
        };

        connections.fetch_add(1, Ordering::Relaxed);
        if connections.load(Ordering::Relaxed) == config.limits.max_devices {
            warn!("Reached maximum number of connections, new connections will be blocked");
        }

        {
            let config = Arc::clone(&config);
            let connections = connections.clone();
            let db = shared_db.clone();

            debug!("Connection count: {}", connections.load(Ordering::SeqCst));

            thread::spawn(move || {
                debug!("New client: {}", peer_addr);

                match handle_client(client, &db, connections.clone(), config) {
                    Ok(()) => {
                        debug!("{}: Handled successfully", peer_addr);
                    }
                    Err(why) => {
                        error!("{peer_addr}: {why}");
                    }
                }

                connections.fetch_sub(1, Ordering::Relaxed);
            });
        }
    }
}

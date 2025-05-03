use super::{config::Config, db::DatabaseClient, rate_limit::RateLimiter};
use crate::server::client_handle::handle_client;
use log::{debug, error, info, warn};
use semaphore::Semaphore;
use std::{net::SocketAddr, panic, sync::Arc, time::Duration};
use tokio::{
    net::{TcpListener, TcpStream},
    runtime::Handle,
    select,
    signal::unix::Signal,
};

#[allow(clippy::needless_pass_by_value)]
pub async fn server_loop(
    server: &TcpListener,
    db: DatabaseClient,
    config: Arc<Config>,
    mut stop_sig: Signal,
    mut ping_sig: Signal,
) {
    let shared_db = Arc::new(db);
    let connections = Semaphore::new(config.limits.devices as _, ());
    let mut rate_limiter = RateLimiter::new(
        Duration::from_secs(config.rate_limits.time_frame),
        config.rate_limits.max_connections,
    );

    loop {
        select! {
            res = server.accept() => {
                match res {
                    Ok(res) => handle_new_client(res.0, res.1, Arc::clone(&shared_db), &connections, &mut rate_limiter, Arc::clone(&config)),
                    Err(why) => {
                        error!("Failed to accept connection: {why}");
                        return;
                    }
                }
            }

            _ = stop_sig.recv() => {
                info!("Stopping server");
                break;
            }

            _ = ping_sig.recv() => {
                info!("Ping requested through SIGUSR1");
                display_rt_metrics();
            }
        }
    }
}

fn handle_new_client(
    client: TcpStream,
    peer_addr: SocketAddr,
    shared_db: Arc<DatabaseClient>,
    connections: &Semaphore<()>,
    rate_limiter: &mut RateLimiter,
    config: Arc<Config>,
) {
    debug!("New client: {peer_addr}");

    debug!("Incrementing connection count");
    let Ok(semguard) = connections.try_access() else {
        warn!("Maximum number of connections reached, ignoring connection");
        return;
    };

    if rate_limiter.hit() {
        warn!("Exceeded rate limit for accepting incoming connections");
        return;
    }

    debug!("{peer_addr:?}: Setting socket parameters");
    if let Err(why) = super::set_global_socket_params(&client) {
        error!("{peer_addr:?}: Failed to set socket parameters: {why}");
        return;
    }

    debug!("Starting client task");
    tokio::spawn(async move {
        let _semguard = semguard;

        debug!("Starting client handle");
        match handle_client(client, peer_addr, &shared_db, config).await {
            Ok(()) => {
                debug!("{peer_addr}: Handled successfully");
            }
            Err(why) => {
                error!("{peer_addr}: Failed to handle: {why}");
            }
        }
    });
}

fn display_rt_metrics() {
    let Ok(handle) = Handle::try_current() else {
        error!("Runtime metrics are not available");
        return;
    };
    let metrics = handle.metrics();

    info!(
        "Pending tasks in the runtime's global queue: {}",
        metrics.global_queue_depth()
    );
    info!("Tasks alive: {}", metrics.num_alive_tasks());
    info!("Workers: {}", metrics.num_workers());
}

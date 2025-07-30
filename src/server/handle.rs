use super::{config::Config, database::DatabaseClient, rate_limit::RateLimiter};
use crate::server::client_handle::handle_client;
use semaphore::Semaphore;
use std::{net::SocketAddr, panic, sync::Arc, time::Duration};
use tokio::{
    net::{TcpListener, TcpStream},
    runtime::Handle,
    select,
    signal::unix::Signal,
};
use tracing::{debug, error, info, warn};

#[allow(clippy::needless_pass_by_value, clippy::cognitive_complexity)]
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

#[allow(clippy::cognitive_complexity)]
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
    match Handle::try_current().as_ref().map(Handle::metrics) {
        Ok(metrics) => {
            info!(
                "Stats: tasks_pending={}, tasks_alive={}, workers={}",
                metrics.global_queue_depth(),
                metrics.num_alive_tasks(),
                metrics.num_workers()
            );
        }
        Err(e) => {
            error!("Runtime metrics are not available: {e}");
        }
    }
}

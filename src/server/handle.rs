use super::{
    client::{Authenticated, Client, Unathenticated},
    config::Config,
    db::DatabaseClient,
    rate_limit::RateLimiter,
};
use log::{debug, error, info, warn};
use message_io::{
    network::NetEvent,
    node::{NodeHandler, NodeListener},
};
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

static CONNECTIONS: AtomicUsize = AtomicUsize::new(0);

#[allow(clippy::needless_pass_by_value)]
pub fn server_loop(
    handler: NodeHandler<()>,
    listener: NodeListener<()>,
    db: DatabaseClient,
    config: Arc<Config>,
) {
    let shared_db = Arc::new(db);
    let mut rate_limiter = RateLimiter::new(
        Duration::from_secs(config.rate_limits.time_frame),
        config.rate_limits.max_connections,
    );
    let mut unanthenticated_clients: Vec<Client<Unathenticated>> = Vec::new();
    let mut authenticated_clients: Vec<Client<Authenticated>> = Vec::new();

    listener.for_each(move |event| match event.network() {
        NetEvent::Connected(endpoint, false) => {
            if CONNECTIONS.load(Ordering::SeqCst) == config.rate_limits.max_connections {
                warn!("Maximum number of connections reached, ignoring connection");
                handler.network().remove(endpoint.resource_id());
            }

            debug!(
                "New endpoint connection '{}', incomplete/failed handshake",
                endpoint.addr()
            );
            CONNECTIONS.fetch_add(1, Ordering::SeqCst);
        }
        NetEvent::Connected(endpoint, true) => debug!(
            "New endpoint connection '{}', completed handshake handshake",
            endpoint.addr()
        ),
        NetEvent::Accepted(endpoint, ..) => {
            unanthenticated_clients.push(Client::new(endpoint, handler.clone()));
        }
        NetEvent::Disconnected(endpoint) => {
            unanthenticated_clients.retain(|candidate| candidate.endpoint() != &endpoint);
            authenticated_clients.retain(|candidate| candidate.endpoint() != &endpoint);
        }
        NetEvent::Message(endpoint, raw_msg) => {
            if rate_limiter.hit() {
                warn!("Request rate limit reached, ignoring message");
                return;
            }

            let unauthenticated_client = unanthenticated_clients
                .iter()
                .enumerate()
                .find(|(_, client)| client.endpoint() == &endpoint)
                .map(|(i, _)| i)
                .and_then(|i| take(&mut unanthenticated_clients, i));

            if let Some(client) = unauthenticated_client {
                let authenticated_client = match client.process(raw_msg, &shared_db) {
                    Ok(client) => client,
                    Err(why) => {
                        error!(
                            "{}: Failed authentication, kicking ({why})",
                            endpoint.addr()
                        );
                        handler.network().remove(endpoint.resource_id());
                        return;
                    }
                };

                authenticated_clients.push(authenticated_client);
                return;
            }

            let authenticated_client = authenticated_clients
                .iter()
                .enumerate()
                .find(|(_, client)| client.endpoint() == &endpoint)
                .map(|(i, _)| i)
                .and_then(|i| take(&mut authenticated_clients, i));

            if let Some(mut client) = authenticated_client {
                match client.process(raw_msg, &shared_db) {
                    Ok(()) => {
                        debug!("{}: Handled successfully", client.id());
                        authenticated_clients.push(client);
                    }
                    Err(why) => {
                        error!("{}: Failed to handle: {why}", client.id());
                        handler.network().remove(client.endpoint().resource_id());
                    }
                }
            }

            error!("{}: Unknown endpoint", endpoint.addr());
        }
    });

    info!("Stop requested");
}

fn take<T>(vec: &mut Vec<T>, index: usize) -> Option<T> {
    if vec.get(index).is_none() {
        None
    } else {
        Some(vec.swap_remove(index))
    }
}

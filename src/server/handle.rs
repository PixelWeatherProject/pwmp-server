use super::{
    client::Client, client_handle, config::Config, db::DatabaseClient, rate_limit::RateLimiter,
};
use crate::error::Error;
use log::{debug, error};
use mio::{Events, Interest, Poll, Token, net::TcpListener};
use std::{
    io::ErrorKind,
    time::{Duration, Instant},
};

const SERVER_TOKEN: Token = Token(0);
const MGMT_TASK_TIMER: Duration = Duration::from_secs(1);

#[allow(clippy::needless_pass_by_value)]
pub fn server_loop(
    mut server: TcpListener,
    mut db: DatabaseClient,
    config: Config,
) -> Result<(), Error> {
    let mut clients: Vec<Client> = Vec::new();
    let mut rate_limiter = RateLimiter::new(
        Duration::from_secs(config.rate_limits.time_frame),
        config.rate_limits.max_requests,
    );
    let mut events = Events::with_capacity(1024);
    let mut poller = Poll::new()?;
    let mut reregister = false;
    let mut last_mgmt_task_run = Instant::now();

    poller
        .registry()
        .register(&mut server, SERVER_TOKEN, Interest::READABLE)?;

    loop {
        if reregister {
            poller
                .registry()
                .reregister(&mut server, SERVER_TOKEN, Interest::READABLE)?;

            for (token, client) in clients.iter_mut().enumerate() {
                poller
                    .registry()
                    .register(client.socket(), Token(token), Interest::READABLE)?;
            }
        }

        let result = poller.poll(&mut events, Some(Duration::from_secs(1)));

        match result {
            Ok(()) => (),
            Err(why) if why.kind() == ErrorKind::TimedOut => (),
            Err(why) => panic!("Polling failed: {why}"),
        }

        /* Handle clients */

        let mut pending_removal = Vec::new();

        for event in &events {
            if !event.is_readable() {
                let client = &clients[event.token().0];
                debug!("I/O error with client: {client:?}");

                pending_removal.push(event.token());
            }

            match event.token() {
                SERVER_TOKEN => {
                    if clients.len() == config.rate_limits.max_connections {
                        error!("Too many clients, not accepting connection");
                    } else {
                        handle_connection_request(&server, &mut clients)
                    }
                }
                other => {
                    match client_handle::handle_client(
                        &mut clients[other.0],
                        &mut db,
                        &mut rate_limiter,
                    ) {
                        Ok(()) => debug!("Client handled successfully"),
                        Err(why) => {
                            error!("Failed to handle client: {why}");
                            pending_removal.push(other);
                        }
                    }
                }
            }
        }

        if !pending_removal.is_empty() {
            debug!("Cleaning up {} clients", pending_removal.len());

            for token in pending_removal {
                clients.remove(token.0);
            }
        }

        handle_mgmt_tasks(&mut last_mgmt_task_run, &mut clients, &config);

        if !reregister {
            reregister = true;
        }
    }
}

fn handle_mgmt_tasks(last_mgmt_task_run: &mut Instant, clients: &mut Vec<Client>, config: &Config) {
    if last_mgmt_task_run.elapsed() <= MGMT_TASK_TIMER {
        return;
    }

    // Kick clients that are stalling
    clients.retain(|client| client.stall_time() <= config.max_stall_time);

    // TODO: Implement more as needed...
}

fn handle_connection_request(server: &TcpListener, clients: &mut Vec<Client>) {
    match server.accept() {
        Ok((stream, _)) => clients.push(Client::new(stream)),
        Err(why) => error!("Failed to accept connection: {why}"),
    }
}

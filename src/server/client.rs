use super::db::{FirmwareBlob, MeasurementId, NodeId};
use crate::error::Error;
use log::warn;
use mio::net::TcpStream;
use pwmp_client::pwmp_msg::{
    Message, mac::Mac, request::Request, response::Response, version::Version,
};
use std::{
    io::{Cursor, Read, Write},
    net::Shutdown,
    time::{Duration, Instant},
};

const RCV_BUFFER_SIZE: usize = 128;
type Result<T> = ::std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Client {
    socket: TcpStream,
    buf: [u8; RCV_BUFFER_SIZE],
    state: ClientState,
    last_interaction: Instant,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ClientState {
    Unathenticated,
    Authenticated {
        id: NodeId,
        mac: Mac,
        last_submit: Option<MeasurementId>,
        update_state: UpdateState,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub enum UpdateState {
    Unchecked,
    UpToDate,
    Available {
        current: Version,
        new: Version,
        blob: Cursor<FirmwareBlob>,
    },
}

impl Client {
    pub fn new(socket: TcpStream) -> Self {
        Self {
            socket,
            buf: [0; RCV_BUFFER_SIZE],
            state: ClientState::Unathenticated,
            last_interaction: Instant::now(),
        }
    }

    /// Some string that identifies the client.
    /// The result depends on whether the client is authenticated or not.
    ///
    /// ### Authenticated clients
    /// The node ID is used as the ID.
    ///
    /// ### Unauthenticated clients
    /// The peer address and port are used.
    /// If this information cannot be requested from the OS, the result will default to
    /// a single question-mark (`?`).
    ///
    pub fn debug_id(&self) -> String {
        match self.state {
            ClientState::Unathenticated => self
                .socket
                .peer_addr()
                .map(|result| format!("{}:{}", result.ip(), result.port()))
                .unwrap_or_else(|_| "?".to_string()),
            ClientState::Authenticated { id, .. } => format!("#{id}"),
        }
    }

    pub fn id(&self) -> Result<NodeId> {
        let ClientState::Authenticated { id, .. } = self.state else {
            warn!("Client::id() called on unauthenticated client");
            return Err(Error::ClientNotAuthenticated);
        };

        Ok(id)
    }

    /// Return a mutable reference to the client's TCP stream.
    pub const fn socket(&mut self) -> &mut TcpStream {
        &mut self.socket
    }

    /// Time since the client has last communicated with the server.
    pub fn stall_time(&self) -> Duration {
        self.last_interaction.elapsed()
    }

    pub fn is_authenticated(&self) -> bool {
        self.state != ClientState::Unathenticated
    }

    pub fn get_hello(&mut self) -> Result<Mac> {
        let message = self.get_request()?;

        let Request::Hello { mac } = message else {
            return Err(Error::NotHello);
        };

        Ok(mac)
    }

    pub fn authorize(&mut self, id: NodeId, mac: Mac) {
        self.state = ClientState::Authenticated {
            id,
            mac,
            last_submit: None,
            update_state: UpdateState::Unchecked,
        }
    }

    pub fn get_request(&mut self) -> Result<Request> {
        self.get_message()?.as_request().ok_or(Error::NotRequest)
    }

    pub fn send_response(&mut self, response: Response) -> Result<()> {
        self.socket
            .write_all(&Message::Response(response).serialize())?;
        Ok(())
    }

    pub fn last_submit(&self) -> Result<Option<MeasurementId>> {
        let ClientState::Authenticated { last_submit, .. } = self.state else {
            warn!("Client::last_submit() called on unauthenticated client");
            return Err(Error::ClientNotAuthenticated);
        };

        Ok(last_submit)
    }

    pub fn set_last_submit(&mut self, id: MeasurementId) -> Result<()> {
        let ClientState::Authenticated {
            ref mut last_submit,
            ..
        } = self.state
        else {
            warn!("Client::set_last_submit() called on unauthenticated client");
            return Err(Error::ClientNotAuthenticated);
        };

        *last_submit = Some(id);
        Ok(())
    }

    fn get_message(&mut self) -> Result<Message> {
        let amount = self.socket.read(&mut self.buf)?;
        Message::deserialize(&self.buf[..amount]).ok_or(Error::MessageParse)
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        if let Err(why) = self.socket.shutdown(Shutdown::Both) {
            warn!("Failed to shut down client socket: {why}");
        }
    }
}

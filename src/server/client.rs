use super::db::{DatabaseClient, FirmwareBlob, MeasurementId, NodeId};
use crate::error::Error;
use log::{debug, warn};
use pwmp_msg::{mac::Mac, request::Request, response::Response, version::Version, Message};
use std::{
    io::{Cursor, Read, Write},
    net::{SocketAddr, TcpStream},
};

const RCV_BUFFER_SIZE: usize = 128;
type Result<T> = ::std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Client<S> {
    socket: TcpStream,
    buf: [u8; RCV_BUFFER_SIZE],
    state: S,
}

#[derive(Debug)]
pub enum UpdateState {
    Unchecked,
    UpToDate,
    Available {
        current: Version,
        new: Version,
        blob: Cursor<FirmwareBlob>,
    },
}

#[derive(Debug)]
pub struct Unathenticated;

#[derive(Debug)]
pub struct Authenticated {
    id: NodeId,
    mac: Mac,
    last_submit: Option<MeasurementId>,
    update_state: UpdateState,
}

impl<S> Client<S> {
    pub fn await_request(&mut self) -> Result<Request> {
        self.await_next_message()?
            .as_request()
            .ok_or(Error::NotRequest)
    }

    fn peer_addr(&self) -> Option<SocketAddr> {
        self.socket.peer_addr().ok()
    }

    pub fn send_response(&mut self, resp: Response) -> Result<()> {
        let message = Message::Response(resp);
        debug!(
            "{}: responding with {:?} ({} bytes)",
            self.peer_addr_str(),
            message.response().unwrap(),
            message.size()
        );
        self.socket.write_all(&message.serialize())?;
        self.socket.flush()?;

        Ok(())
    }

    fn peer_addr_str(&self) -> String {
        self.peer_addr()
            .map_or_else(|| "Unknown".to_string(), |addr| addr.to_string())
    }

    fn await_next_message(&mut self) -> Result<Message> {
        let read = self.socket.read(&mut self.buf)?;
        if read == 0 {
            return Err(Error::Quit);
        }

        let message = Message::deserialize(&self.buf[..read]).ok_or(Error::MessageParse)?;

        Ok(message)
    }
}

impl Client<Unathenticated> {
    pub const fn new(socket: TcpStream) -> Self {
        Self {
            socket,
            buf: [0; RCV_BUFFER_SIZE],
            state: Unathenticated,
        }
    }

    pub fn authorize(mut self, db: &DatabaseClient) -> Result<Client<Authenticated>> {
        debug!("{}: Awaiting greeting", self.peer_addr_str());
        let mac = self.handle_hello()?;

        debug!("{}: Is {}?", self.peer_addr_str(), mac);

        match db.authorize_device(&mac) {
            Ok(Some(id)) => {
                let mut authorized_client = Client::<Authenticated>::new(self, id, mac);
                debug!(
                    "Device {} authorized as node #{id}",
                    authorized_client.mac()
                );

                authorized_client.send_response(Response::Ok)?;
                Ok(authorized_client)
            }
            Ok(None) => Err(Error::Auth),
            Err(why) => {
                warn!("Device {} is not authorized", self.peer_addr_str());
                self.send_response(Response::Reject)?;
                Err(why)
            }
        }
    }

    fn handle_hello(&mut self) -> Result<Mac> {
        let req = self.await_request()?;
        let Request::Hello { mac } = req else {
            return Err(Error::NotHello);
        };

        Ok(mac)
    }
}

impl Client<Authenticated> {
    fn new(client: Client<Unathenticated>, id: NodeId, mac: Mac) -> Self {
        Self {
            socket: client.socket,
            buf: client.buf,
            state: Authenticated {
                id,
                mac,
                last_submit: None,
                update_state: UpdateState::Unchecked,
            },
        }
    }

    pub const fn id(&self) -> NodeId {
        self.state.id
    }

    pub const fn mac(&self) -> &Mac {
        &self.state.mac
    }

    pub const fn last_submit(&self) -> Option<MeasurementId> {
        self.state.last_submit
    }

    pub const fn set_last_submit(&mut self, value: MeasurementId) {
        self.state.last_submit = Some(value);
    }

    pub fn mark_up_to_date(&mut self) {
        self.state.update_state = UpdateState::UpToDate;
    }

    pub fn store_update_check_result(
        &mut self,
        current_version: Version,
        new_version: Version,
        blob: FirmwareBlob,
    ) {
        self.state.update_state = UpdateState::Available {
            current: current_version,
            new: new_version,
            blob: Cursor::new(blob),
        };
    }

    pub const fn update_chunk(&mut self) -> Option<&mut Cursor<Box<[u8]>>> {
        match self.state.update_state {
            UpdateState::Available { ref mut blob, .. } => Some(blob),
            _ => None,
        }
    }

    pub const fn current_version(&self) -> Option<Version> {
        match self.state.update_state {
            UpdateState::Available { current, .. } => Some(current),
            _ => None,
        }
    }

    pub const fn update_version(&self) -> Option<Version> {
        match self.state.update_state {
            UpdateState::Available { new, .. } => Some(new),
            _ => None,
        }
    }
}

use super::db::{DatabaseClient, FirmwareBlob, MeasurementId, NodeId};
use crate::error::Error;
use arrayref::array_ref;
use log::{debug, error, warn};
use pwmp_client::pwmp_msg::{
    Message, MsgId, mac::Mac, request::Request, response::Response, version::Version,
};
use std::{
    io::{Cursor, Read, Write},
    net::{Shutdown, SocketAddr, TcpStream},
};

const RCV_BUFFER_SIZE: usize = 128;
const ID_CACHE_SIZE: usize = 32;
type Result<T> = ::std::result::Result<T, Error>;
type MsgLength = u32;

pub struct Client<S> {
    socket: TcpStream,
    buf: [u8; RCV_BUFFER_SIZE],
    id_cache: [MsgId; ID_CACHE_SIZE],
    last_id: MsgId,
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
    pub fn shutdown(&mut self, reason: Option<Response>) -> Result<()> {
        debug!("{}: Attempting to shutdown socket", self.peer_addr_str());

        if let Some(res) = reason {
            debug!("{}: Sending reason code", self.peer_addr_str());
            if let Err(why) = self.send_response(res) {
                warn!("{}: Failed to send: {why}", self.peer_addr_str());
            }
        }

        self.socket.shutdown(Shutdown::Both)?;
        Ok(())
    }

    fn peer_addr(&self) -> Option<SocketAddr> {
        self.socket.peer_addr().ok()
    }

    fn peer_addr_str(&self) -> String {
        self.peer_addr()
            .map_or_else(|| "Unknown".to_string(), |addr| addr.to_string())
    }

    pub fn send_response(&mut self, res: Response) -> Result<()> {
        debug!("{}: responding with {res:?}", self.peer_addr_str(),);
        self.last_id += 1;
        self.send_message(Message::new_response(res, self.last_id))
    }

    pub fn receive_request(&mut self) -> Result<Request> {
        // Receive the message.
        let message = self.receive_message()?;

        // Convert it to a request.
        message.take_request().ok_or(Error::NotRequest)
    }

    fn send_message(&mut self, msg: Message) -> Result<()> {
        // Serialize the message.
        let raw = msg.serialize();

        // Get the length and store it as a proper length integer.
        let length: MsgLength = raw.len().try_into().map_err(|_| Error::MessageTooLarge)?;

        // Send the length first as big/network endian.
        self.socket.write_all(length.to_be_bytes().as_slice())?;

        // Send the actual message next.
        // TODO: Endianness should be handled internally, but this should be checked!
        self.socket.write_all(&raw)?;

        // Flush the buffer.
        self.socket.flush()?;

        // Done
        Ok(())
    }

    fn receive_message(&mut self) -> Result<Message> {
        // First read the message size.
        self.socket
            .read_exact(&mut self.buf[..size_of::<MsgLength>()])?;

        // Parse the length
        let message_length: usize =
            u32::from_be_bytes(*array_ref![self.buf, 0, size_of::<u32>()]).try_into()?;

        // Verify the length
        if message_length == 0 {
            return Err(Error::IllegalMessageLength);
        }
        // TODO: Add more restrictions as needed...

        // Verify that the buffer is large enough.
        if self.buf.len() < message_length {
            return Err(Error::InvalidBuffer);
        }

        // Read the actual message.
        self.socket.read_exact(&mut self.buf[..message_length])?;

        // Parse the message.
        let message = Message::deserialize(&self.buf).ok_or(Error::MessageParse)?;

        // Check if it's not a duplicate.
        if self.is_id_cached(message.id()) {
            return Err(Error::DuplicateMessage);
        }

        // Cache the ID.
        self.cache_id(message.id());

        // Done
        Ok(message)
    }

    fn is_id_cached(&self, id: MsgId) -> bool {
        // Check if the ID matches any of the cached ones.
        self.id_cache.iter().any(|candidate| candidate == &id)
    }

    fn cache_id(&mut self, id: MsgId) {
        // Rotate the array left by one.
        self.id_cache.rotate_left(1);

        // Set the last ID to the specified one.
        self.id_cache[self.id_cache.len() - 1] = id;
    }
}

impl Client<Unathenticated> {
    pub const fn new(socket: TcpStream) -> Self {
        Self {
            socket,
            buf: [0; RCV_BUFFER_SIZE],
            id_cache: [0; ID_CACHE_SIZE],
            last_id: 1,
            state: Unathenticated,
        }
    }

    pub fn authorize(mut self, db: &DatabaseClient) -> Result<Client<Authenticated>> {
        debug!("{}: Awaiting greeting", self.peer_addr_str());
        let mac = self.receive_handshake()?;

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
            Ok(None) => {
                warn!("Device {} is not authorized", self.peer_addr_str());
                self.send_response(Response::Reject)?;
                Err(Error::Auth)
            }
            Err(why) => {
                error!(
                    "Could not perform authentication of {}",
                    self.peer_addr_str()
                );
                self.send_response(Response::Reject)?;
                Err(why)
            }
        }
    }

    fn receive_handshake(&mut self) -> Result<Mac> {
        let req = self.receive_request()?;
        let Request::Handshake { mac } = req else {
            return Err(Error::NotHandshake);
        };

        Ok(mac)
    }
}

impl Client<Authenticated> {
    fn new(client: Client<Unathenticated>, id: NodeId, mac: Mac) -> Self {
        Self {
            socket: client.socket,
            buf: client.buf,
            id_cache: [0; ID_CACHE_SIZE],
            last_id: client.last_id,
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

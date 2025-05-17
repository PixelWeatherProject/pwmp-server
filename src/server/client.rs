use super::db::{DatabaseClient, FirmwareBlob, MeasurementId, NodeId};
use crate::error::Error;
use arrayref::array_ref;
use circular_queue::CircularQueue;
use pwmp_client::pwmp_msg::{
    Message, MsgId, mac::Mac, request::Request, response::Response, version::Version,
};
use std::{io::Cursor, net::SocketAddr};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
use tracing::{debug, error, warn};

const RCV_BUFFER_SIZE: usize = 1024;
const ID_CACHE_SIZE: usize = 32;
type Result<T> = ::std::result::Result<T, Error>;
type MsgLength = u32;

pub struct Client<S> {
    stream: TcpStream,
    buf: [u8; RCV_BUFFER_SIZE],
    id_cache: CircularQueue<MsgId>,
    last_id: MsgId,
    state: S,
    peer_addr: Box<str>,
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
    pub async fn send_response(&mut self, res: Response) -> Result<()> {
        debug!("{}: responding with {res:?}", self.peer_addr);
        self.last_id += 1;
        self.send_message(Message::new_response(res, self.last_id))
            .await
    }

    pub async fn receive_request(&mut self) -> Result<Request> {
        // Receive the message.
        let message = self.receive_message().await?;

        // Convert it to a request.
        message.take_request().ok_or(Error::NotRequest)
    }

    async fn send_message(&mut self, msg: Message) -> Result<()> {
        // Serialize the message.
        let raw = msg.serialize();

        // Get the length and store it as a proper length integer.
        let length: MsgLength = raw.len().try_into().map_err(|_| Error::MessageTooLarge)?;

        // Send the length first as big/network endian.
        self.stream
            .write_all(length.to_be_bytes().as_slice())
            .await?;

        // Send the actual message next.
        // TODO: Endianness should be handled internally, but this should be checked!
        self.stream.write_all(&raw).await?;

        // Flush the buffer.
        self.stream.flush().await?;

        // Done
        Ok(())
    }

    async fn receive_message(&mut self) -> Result<Message> {
        // First read the message size.
        self.stream
            .read_exact(&mut self.buf[..size_of::<MsgLength>()])
            .await?;

        // Parse the length
        let message_length: usize =
            MsgLength::from_be_bytes(*array_ref![self.buf, 0, size_of::<MsgLength>()])
                .try_into()?;

        // Verify the length
        if message_length == 0 {
            return Err(Error::IllegalMessageLength);
        }
        // TODO: Add more restrictions as needed...

        // Verify that the buffer is large enough.
        if self.buf.len() < message_length {
            error!(
                "{}: Received message length is {message_length} bytes, but the buffer is only {} bytes",
                self.peer_addr,
                self.buf.len()
            );
            return Err(Error::InvalidBuffer);
        }

        // Read the actual message.
        self.stream
            .read_exact(&mut self.buf[..message_length])
            .await?;

        // Parse the message.
        let message =
            Message::deserialize(&self.buf[..message_length]).ok_or(Error::MessageParse)?;

        // Check if it's not a duplicate.
        if self.is_id_cached(message.id()) {
            return Err(Error::DuplicateMessage);
        }

        // Cache the ID.
        self.id_cache.push(message.id());

        // Done
        Ok(message)
    }

    fn is_id_cached(&self, id: MsgId) -> bool {
        // Check if the ID matches any of the cached ones.
        self.id_cache.iter().any(|candidate| candidate == &id)
    }
}

impl Client<Unathenticated> {
    pub fn new(socket: TcpStream, peer_addr: SocketAddr) -> Self {
        Self {
            stream: socket,
            buf: [0; RCV_BUFFER_SIZE],
            id_cache: CircularQueue::with_capacity(ID_CACHE_SIZE),
            last_id: 1,
            state: Unathenticated,
            peer_addr: peer_addr.to_string().into_boxed_str(),
        }
    }

    pub async fn authorize(mut self, db: &DatabaseClient) -> Result<Client<Authenticated>> {
        debug!("{}: Awaiting greeting", self.peer_addr);
        let mac = self.receive_handshake().await?;

        debug!("{}: Is {}?", self.peer_addr, mac);

        match db.authorize_device(&mac).await {
            Ok(Some(id)) => {
                let mut authorized_client = Client::<Authenticated>::new(self, id, mac);
                debug!(
                    "Device {} authorized as node #{id}",
                    authorized_client.mac()
                );

                authorized_client.send_response(Response::Ok).await?;
                Ok(authorized_client)
            }
            Ok(None) => {
                warn!("Device {} is not authorized", self.peer_addr);
                self.send_response(Response::Reject).await?;
                Err(Error::Auth)
            }
            Err(why) => {
                error!("Could not perform authentication of {}", self.peer_addr);
                self.send_response(Response::Reject).await?;
                Err(why)
            }
        }
    }

    async fn receive_handshake(&mut self) -> Result<Mac> {
        let req = self.receive_request().await?;
        let Request::Handshake { mac } = req else {
            return Err(Error::NotHandshake);
        };

        Ok(mac)
    }
}

impl Client<Authenticated> {
    fn new(client: Client<Unathenticated>, id: NodeId, mac: Mac) -> Self {
        Self {
            stream: client.stream,
            buf: client.buf,
            id_cache: client.id_cache,
            last_id: client.last_id,
            state: Authenticated {
                id,
                mac,
                last_submit: None,
                update_state: UpdateState::Unchecked,
            },
            peer_addr: client.peer_addr,
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

    // Note: This method could be implemented for Client<S>, but that would make it impossible
    //       to print the Node ID. This way it's possible and makes debugging slightly easier.
    pub async fn shutdown(&mut self, reason: Option<Response>) -> Result<()> {
        debug!("{}: Attempting to shutdown socket", self.id());

        if let Some(res) = reason {
            debug!("{}: Sending reason code", self.id());
            if let Err(why) = self.send_response(res).await {
                warn!("{}: Failed to send: {why}", self.id());
            }
        }

        self.stream.shutdown().await?;
        debug!("{}: Stream shut down", self.id());
        Ok(())
    }
}

use super::db::{DatabaseClient, FirmwareBlob, MeasurementId, NodeId};
use crate::{error::Error, server::db::DatabaseBackend};
use pwmp_client::pwmp_msg::{
    Message, MsgId, mac::Mac, request::Request, response::Response, settings::NodeSettings,
    version::Version,
};
use std::{io::IoSlice, net::SocketAddr, vec::IntoIter};
use tokio::{
    io::{self, AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
use tracing::{debug, error, warn};

const RCV_BUFFER_SIZE: usize = 4096;
type Result<T> = ::std::result::Result<T, Error>;
type MsgLength = u32;

pub struct Client<S> {
    stream: TcpStream,
    buf: Box<[u8]>,
    id: MsgId,
    last_id: Option<MsgId>,
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
        blob: IntoIter<u8>,
    },
    Done {
        current: Version,
        new: Version,
    },
}

#[derive(Debug)]
pub struct Unathenticated;

#[derive(Debug)]
pub struct Authenticated {
    id: NodeId,
    mac: Mac,
    os_version: Version,
    os_version_ext: Option<Box<str>>,
    last_submit: Option<MeasurementId>,
    update_state: UpdateState,
    settings: Option<NodeSettings>,
}

impl<S> Client<S> {
    pub async fn send_response(&mut self, res: Response) -> Result<()> {
        debug!("{}: responding with {res:?}", self.peer_addr);

        self.id += 1;
        self.send_message(Message::new_response(res, self.id)).await
    }

    pub async fn receive_request(&mut self) -> Result<Request> {
        // Receive the message.
        let message = self.receive_message().await?;

        // Convert it to a request.
        message.take_request().ok_or(Error::NotRequest)
    }

    #[tracing::instrument(name = "Client::send_message()", skip(self), level = "debug")]
    async fn send_message(&mut self, msg: Message) -> Result<()> {
        // Serialize the message.
        let raw = msg.serialize()?;

        // Get the length and store it as a proper length integer.
        let length: MsgLength = raw.len().try_into().map_err(|_| Error::MessageTooLarge)?;
        let header = length.to_be_bytes();

        // Prepare the complete message for a vectored write
        let mut slices = &mut [IoSlice::new(&header), IoSlice::new(&raw)][..];

        // Write the entire message to the stocket
        while !slices.is_empty() {
            let written = self.stream.write_vectored(slices).await?;

            if written == 0 {
                return Err(io::Error::from(io::ErrorKind::WriteZero).into());
            }

            // Skip past whatever was written, in case the kernel accepted only part of it.
            IoSlice::advance_slices(&mut slices, written);
        }

        // Flush the buffer.
        self.stream.flush().await?;

        // Done
        Ok(())
    }

    #[tracing::instrument(
        name = "Client::receive_message()",
        skip(self),
        level = "debug",
        err,
        ret
    )]
    async fn receive_message(&mut self) -> Result<Message> {
        // First read the message size.
        self.stream
            .read_exact(&mut self.buf[..size_of::<MsgLength>()])
            .await?;

        // Parse the length
        let message_length: usize =
            MsgLength::from_be_bytes(self.buf[..size_of::<MsgLength>()].try_into()?).try_into()?;

        // Verify the length
        if message_length == 0 {
            return Err(Error::IllegalMessageLength);
        }

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
        if self.check_id_conflict(message.id()) {
            return Err(Error::DuplicateMessage);
        }

        // Cache the ID.
        self.last_id = Some(message.id());

        // Done
        Ok(message)
    }

    fn check_id_conflict(&self, id: MsgId) -> bool {
        self.last_id == Some(id)
    }
}

impl Client<Unathenticated> {
    pub fn new(socket: TcpStream, peer_addr: SocketAddr) -> Self {
        Self {
            stream: socket,
            buf: vec![0; RCV_BUFFER_SIZE].into_boxed_slice(),
            id: 0,
            last_id: None,
            state: Unathenticated,
            peer_addr: peer_addr.to_string().into_boxed_str(),
        }
    }

    #[allow(clippy::cognitive_complexity)]
    pub async fn authorize(mut self, db: &DatabaseClient) -> Result<Client<Authenticated>> {
        debug!("{}: Awaiting greeting", self.peer_addr);
        let (mac, os_version, os_version_ext) = self.receive_handshake().await?;

        debug!("{}: Is {}?", self.peer_addr, mac);

        match db.authorize_device(&mac).await {
            Ok(Some(id)) => {
                let mut authorized_client =
                    Client::<Authenticated>::new(self, id, mac, os_version, os_version_ext);
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

    #[tracing::instrument(
        name = "Client<Unathenticated>::receive_handshake()",
        skip(self),
        level = "debug",
        err,
        ret
    )]
    async fn receive_handshake(&mut self) -> Result<(Mac, Version, Option<Box<str>>)> {
        let req = self.receive_request().await?;
        let Request::Hello {
            mac,
            os_version,
            os_version_ext,
        } = req
        else {
            return Err(Error::NotHandshake);
        };

        debug!(
            "{}: Running OS version {os_version} ({})",
            self.peer_addr,
            os_version_ext.as_deref().unwrap_or("-")
        );

        Ok((mac, os_version, os_version_ext))
    }
}

impl Client<Authenticated> {
    fn new(
        client: Client<Unathenticated>,
        id: NodeId,
        mac: Mac,
        os_version: Version,
        os_version_ext: Option<Box<str>>,
    ) -> Self {
        Self {
            stream: client.stream,
            buf: client.buf,
            id: client.id,
            last_id: client.last_id,
            state: Authenticated {
                id,
                mac,
                os_version,
                os_version_ext,
                last_submit: None,
                update_state: UpdateState::Unchecked,
                settings: None,
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

    pub const fn os_version(&self) -> Version {
        self.state.os_version
    }

    pub fn os_version_ext(&self) -> Option<&str> {
        self.state.os_version_ext.as_deref()
    }

    pub const fn last_submit(&self) -> Option<MeasurementId> {
        self.state.last_submit
    }

    pub fn mark_up_to_date(&mut self) {
        self.state.update_state = UpdateState::UpToDate;
    }

    pub async fn get_settings(&mut self, db: &DatabaseClient) -> Result<Option<NodeSettings>> {
        if let Some(settings) = self.state.settings {
            Ok(Some(settings))
        } else {
            let settings = db.get_settings(self.id()).await?;
            self.state.settings = settings;
            Ok(settings)
        }
    }

    pub fn notifications_muted(&self) -> bool {
        // Note: This assumes that `get_settings()` has been called at least once
        self.state.settings.map_or_else(
            || NodeSettings::default().mute_notifications,
            |s| s.mute_notifications,
        )
    }

    #[tracing::instrument(
        name = "Client<Authenticated>::store_update_check_result()",
        skip(self, blob),
        level = "debug"
    )]
    pub fn store_update_check_result(
        &mut self,
        current_version: Version,
        new_version: Version,
        blob: FirmwareBlob,
    ) {
        self.state.update_state = UpdateState::Available {
            current: current_version,
            new: new_version,
            blob: blob.into_iter(),
        };
    }

    pub fn next_update_chunk(&mut self, max_size: u32) -> Option<FirmwareBlob> {
        let UpdateState::Available { blob, .. } = &mut self.state.update_state else {
            return None;
        };

        let part = blob
            .by_ref()
            .take(usize::try_from(max_size).expect("`max_size` too large to convert to usize"))
            .collect::<FirmwareBlob>();

        Some(part)
    }

    pub fn clear_update_blob(&mut self) {
        if let UpdateState::Available { current, new, .. } = self.state.update_state {
            self.state.update_state = UpdateState::Done { current, new };
        }
    }

    pub const fn current_version(&self) -> Option<Version> {
        match self.state.update_state {
            UpdateState::Available { current, .. } | UpdateState::Done { current, .. } => {
                Some(current)
            }
            _ => None,
        }
    }

    pub const fn update_version(&self) -> Option<Version> {
        match self.state.update_state {
            UpdateState::Available { new, .. } | UpdateState::Done { new, .. } => Some(new),
            _ => None,
        }
    }

    // Note: This method could be implemented for Client<S>, but that would make it impossible
    //       to print the Node ID. This way it's possible and makes debugging slightly easier.
    #[allow(clippy::cognitive_complexity)]
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

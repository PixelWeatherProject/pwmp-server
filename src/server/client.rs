use super::db::{DatabaseClient, FirmwareBlob, MeasurementId, NodeId};
use crate::{
    error::{Error, SendStatusEx},
    server::client_handle::handle_request,
};
use derive_more::Debug;
use log::{debug, error, warn};
use message_io::{network::Endpoint, node::NodeHandler};
use pwmp_client::pwmp_msg::{
    Message, mac::Mac, request::Request, response::Response, version::Version,
};
use std::{io::Cursor, net::SocketAddr};

type Result<T> = ::std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Client<S> {
    #[debug(skip)]
    handler: NodeHandler<()>,
    endpoint: Endpoint,
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
    pub const fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }

    fn peer_addr(&self) -> SocketAddr {
        self.endpoint.addr()
    }

    pub fn send_response(&self, resp: Response) -> Result<()> {
        let message = Message::Response(resp);
        debug!(
            "{}: responding with {:?} ({} bytes)",
            self.peer_addr_str(),
            message.response().unwrap(),
            message.size()
        );

        self.handler
            .network()
            .send(self.endpoint, &message.serialize())
            .errorize()?;
        debug!("Response sent");

        Ok(())
    }

    fn peer_addr_str(&self) -> String {
        self.peer_addr().to_string()
    }
}

impl Client<Unathenticated> {
    pub const fn new(endpoint: Endpoint, handler: NodeHandler<()>) -> Self {
        Self {
            endpoint,
            handler,
            state: Unathenticated,
        }
    }

    pub fn process(self, raw_msg: &[u8], db: &DatabaseClient) -> Result<Client<Authenticated>> {
        debug!("{}: Expecting greeting", self.peer_addr_str());
        let message = Message::deserialize(raw_msg).ok_or(Error::MessageParse)?;
        let request = message.as_request().ok_or(Error::NotRequest)?;
        let Request::Hello { mac } = request else {
            error!(
                "{}: Expected greeting, got {request:?}",
                self.peer_addr_str()
            );
            return Err(Error::NotHello);
        };

        debug!("{}: Is {}?", self.peer_addr_str(), mac);
        match db.authorize_device(&mac) {
            Ok(Some(id)) => {
                let authorized_client = Client::<Authenticated>::new(self, id, mac);
                debug!(
                    "{}: Authorized as node #{id} with MAC {}",
                    authorized_client.peer_addr_str(),
                    authorized_client.mac()
                );

                authorized_client.send_response(Response::Ok)?;
                Ok(authorized_client)
            }
            Ok(None) => {
                warn!(
                    "{}: Failed to authenticate, unknown address: {mac}",
                    self.peer_addr_str()
                );
                self.send_response(Response::Reject)?;
                Err(Error::Auth)
            }
            Err(why) => {
                error!(
                    "{}: Could not perform authentication: {why}",
                    self.peer_addr_str()
                );
                self.send_response(Response::Reject)?;
                Err(why)
            }
        }
    }
}

impl Client<Authenticated> {
    fn new(client: Client<Unathenticated>, id: NodeId, mac: Mac) -> Self {
        Self {
            endpoint: client.endpoint,
            handler: client.handler,
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

    pub fn process(&mut self, raw_msg: &[u8], db: &DatabaseClient) -> Result<()> {
        let message = Message::deserialize(raw_msg).ok_or(Error::MessageParse)?;
        let request = message.as_request().ok_or(Error::NotRequest)?;

        let response = handle_request(request, self, db)?.ok_or(Error::BadRequest)?;
        self.send_response(response)
    }
}

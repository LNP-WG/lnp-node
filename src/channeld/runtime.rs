// LNP Node: node running lightning network protocol and generalized lightning
// channels.
// Written in 2020-2022 by
//     Dr. Maxim Orlovsky <orlovsky@lnp-bp.org>
//
// To the extent possible under law, the author(s) have dedicated all
// copyright and related and neighboring rights to this software to
// the public domain worldwide. This software is distributed without
// any warranty.
//
// You should have received a copy of the MIT License along with this software.
// If not, see <https://opensource.org/licenses/MIT>.

use std::io::Seek;
use std::time::SystemTime;
use std::{fs, io};

use amplify::{DumbDefault, Wrapper};
use internet2::addr::NodeAddr;
use lnp::channel::bolt;
use lnp::p2p::bolt::{ActiveChannelId, ChannelId, Messages as LnMsg};
use lnp::Extension;
use lnp_rpc::{ChannelInfo, RpcMsg};
use microservices::esb::{self, Handler};
use strict_encoding::{StrictDecode, StrictEncode};

use super::storage::{self, Driver};
use super::ChannelState;
use crate::bus::{self, BusMsg, CtlMsg, ServiceBus};
use crate::routed::PaymentError;
use crate::rpc::{ClientId, ServiceId};
use crate::{channeld, Config, Endpoints, Error, Responder, Service};

pub fn run(config: Config, channel_id: ActiveChannelId) -> Result<(), Error> {
    // TODO: use node configuration to provide custom policy & parameters

    // check and read channel file
    let channel_file = config.channel_file(channel_id);
    let (state, file) =
        if let Ok(file) = fs::OpenOptions::new().read(true).write(true).open(&channel_file) {
            debug!("Restoring channel state from {}", channel_file.display());
            let state = ChannelState::strict_decode(&file).map_err(Error::Persistence)?;
            info!("Channel state is restored from persistent storage");
            let mut inner_state = bolt::ChannelState::dumb_default();
            state.channel.store_state(&mut inner_state);
            trace!("Restored state: {}", inner_state);
            (state, file)
        } else if let Some(temp_channel_id) = channel_id.temp_channel_id() {
            debug!("Establishing channel de novo");
            let state = ChannelState::with(temp_channel_id, &config.chain);
            fs::create_dir_all(config.channel_dir())?;
            let file = fs::File::create(channel_file)?;
            (state, file)
        } else {
            error!(
                "Requested to re-establish channel {}, but its state has not persisted on disk. \
                 You may compose a channel",
                channel_id
            );
            return Err(Error::Channel(channeld::Error::NoPersistantData));
        };

    let channel_id = ChannelId::from_inner(channel_id.as_slice32());
    let runtime = Runtime {
        identity: ServiceId::Channel(channel_id),
        config: config.clone(),
        state,
        file,
        started: SystemTime::now(),
        enquirer: None,
        storage: Box::new(storage::DiskDriver::init(
            channel_id,
            Box::new(storage::DiskConfig { path: Default::default() }),
        )?),
    };

    Service::run(config, runtime, false)
}

pub struct Runtime {
    identity: ServiceId,
    config: Config,
    pub(super) state: ChannelState,
    pub(super) file: fs::File,
    started: SystemTime,
    /// Client which is made an equiry starting the current workflow run by the active state
    /// machine. It is not a part of the state of the machine since it should not persist.
    enquirer: Option<ClientId>,
    storage: Box<dyn storage::Driver>,
}

impl Responder for Runtime {
    #[inline]
    fn enquirer(&self) -> Option<ClientId> { self.enquirer }
}

impl esb::Handler<ServiceBus> for Runtime {
    type Request = BusMsg;
    type Error = Error;

    fn identity(&self) -> ServiceId { self.identity.clone() }

    fn handle(
        &mut self,
        endpoints: &mut Endpoints,
        bus: ServiceBus,
        source: ServiceId,
        message: BusMsg,
    ) -> Result<(), Self::Error> {
        match (bus, message, source) {
            (ServiceBus::Msg, BusMsg::Bolt(msg), ServiceId::PeerBolt(remote_peer)) => {
                self.handle_p2p(endpoints, remote_peer, msg)
            }
            (ServiceBus::Msg, BusMsg::Bolt(_), service) => {
                unreachable!("channeld received peer message not from a peerd but from {}", service)
            }
            (ServiceBus::Ctl, BusMsg::Ctl(msg), source) => self.handle_ctl(endpoints, source, msg),
            (ServiceBus::Rpc, BusMsg::Rpc(msg), ServiceId::Client(client_id)) => {
                self.handle_rpc(endpoints, client_id, msg)
            }
            (ServiceBus::Rpc, BusMsg::Rpc(_), service) => {
                unreachable!("lnpd received RPC message not from a client but from {}", service)
            }
            (bus, msg, _) => Err(Error::wrong_esb_msg(bus, &msg)),
        }
    }

    fn handle_err(
        &mut self,
        _: &mut Endpoints,
        _: esb::Error<ServiceId>,
    ) -> Result<(), Self::Error> {
        // We do nothing and do not propagate error; it's already being reported
        // with `error!` macro by the controller. If we propagate error here
        // this will make whole daemon panic
        Ok(())
    }
}

impl Runtime {
    pub(super) fn set_identity(
        &mut self,
        endpoints: &mut Endpoints,
        channel_id: ChannelId,
    ) -> Result<(), Error> {
        let prev_id = self.state.channel.active_channel_id();

        self.file.sync_all()?;
        let file_name = self.config.channel_file(ActiveChannelId::Static(channel_id));
        self.file = fs::File::create(file_name)?;
        self.save_state().map_err(Error::Persistence)?;

        let identity = ServiceId::Channel(channel_id);
        endpoints.set_identity(ServiceBus::Ctl, identity.clone())?;
        endpoints.set_identity(ServiceBus::Msg, identity.clone())?;
        endpoints.set_identity(ServiceBus::Rpc, identity.clone())?;
        self.identity = identity;

        fs::remove_file(self.config.channel_file(prev_id))?;

        Ok(())
    }

    pub fn send_p2p(
        &self,
        endpoints: &mut Endpoints,
        message: LnMsg,
    ) -> Result<(), esb::Error<ServiceId>> {
        let remote_peer = self.state.remote_peer.clone().expect("unset remote peer in channeld");
        endpoints.send_to(
            ServiceBus::Msg,
            self.identity(),
            ServiceId::PeerBolt(remote_peer),
            BusMsg::Bolt(message),
        )
    }

    fn handle_p2p(
        &mut self,
        endpoints: &mut Endpoints,
        remote_peer: NodeAddr,
        message: LnMsg,
    ) -> Result<(), Error> {
        match message {
            LnMsg::OpenChannel(_) => {
                // TODO: Support repeated messages according to BOLT-2 requirements:
                //       If the connection has been re-established after receiving a previous
                //       open_channel, BUT before receiving a funding_created message:
                //       - accept a new open_channel message;
                //       - discard the previous open_channel message.
                warn!(
                    "Got `open_channel` P2P message from {}, which is unexpected: the channel \
                     creation was already requested before",
                    remote_peer
                );
            }

            LnMsg::ChannelReestablish(_)
            | LnMsg::AcceptChannel(_)
            | LnMsg::FundingSigned(_)
            | LnMsg::FundingLocked(_) => {
                self.process(endpoints, ServiceId::PeerBolt(remote_peer), BusMsg::Bolt(message))?;
            }

            _ => {
                // Ignore the rest of LN peer messages
            }
        }
        Ok(())
    }

    fn handle_ctl(
        &mut self,
        endpoints: &mut Endpoints,
        source: ServiceId,
        request: CtlMsg,
    ) -> Result<(), Error> {
        // RPC control requests are sent by either clients or lnpd daemon and used to initiate one
        // of channel workflows and to request information about the channel state.
        match request {
            // Proposing remote peer to open a channel
            CtlMsg::OpenChannelWith(ref open_channel_with) => {
                let remote_peer = open_channel_with.remote_peer.clone();
                self.enquirer = open_channel_with.report_to;
                // Updating state only if the request was processed
                self.state.remote_peer = Some(remote_peer);
                self.process(endpoints, source, BusMsg::Ctl(request))?;
            }

            // Processing remote request to open a channel
            CtlMsg::AcceptChannelFrom(bus::AcceptChannelFrom { ref remote_peer, .. }) => {
                self.enquirer = None;
                let remote_peer = remote_peer.clone();
                if self.process(endpoints, source, BusMsg::Ctl(request))? {
                    // Updating state only if the request was processed
                    self.state.remote_peer = Some(remote_peer);
                }
            }

            CtlMsg::FundingConstructed(_)
            | CtlMsg::TxFound(_)
            | CtlMsg::Signed(_)
            | CtlMsg::Error { .. }
            | CtlMsg::EsbError { .. } => {
                self.process(endpoints, source, BusMsg::Ctl(request))?;
            }

            CtlMsg::Payment { route, hash_lock, enquirer } => {
                // TODO: Move into a state machine
                self.enquirer = Some(enquirer);
                let payment = &route.get(0).ok_or(PaymentError::RouteNotFound)?.payload;
                let message = self.state.channel.compose_add_update_htlc(
                    payment.amt_to_forward,
                    hash_lock,
                    payment.outgoing_cltv_value,
                    route,
                )?;
                self.send_p2p(endpoints, message)?;
                // TODO: Report progress here, wait for new commitment to be signed before reporting
                //       success. Do not clear enquirer
                let _ = self.report_success(endpoints, Some("HTLC added to the channel"));
                self.enquirer = None;
            }

            wrong_request => {
                error!("Request is not supported by the CTL interface");
                return Err(Error::wrong_esb_msg(ServiceBus::Ctl, &wrong_request));
            }
        }
        Ok(())
    }

    fn handle_rpc(
        &mut self,
        endpoints: &mut Endpoints,
        client_id: ClientId,
        request: RpcMsg,
    ) -> Result<(), Error> {
        match request {
            RpcMsg::GetInfo => {
                let mut state = bolt::ChannelState::dumb_default();
                self.state.channel.store_state(&mut state);
                let channel_info =
                    ChannelInfo { state, remote_peer: self.state.remote_peer.clone() };
                self.send_rpc(endpoints, client_id, channel_info)?;
            }
            RpcMsg::Send(_) => todo!("payments are not yet implemented"),
            wrong_request => {
                error!("Request is not supported by the RPC interface");
                return Err(Error::wrong_esb_msg(ServiceBus::Rpc, &wrong_request));
            }
        }

        Ok(())
    }

    // TODO: Use storage drivers
    pub fn save_state(&mut self) -> Result<(), strict_encoding::Error> {
        self.file.seek(io::SeekFrom::Start(0))?;
        self.state.strict_encode(&self.file)?;
        self.file.sync_all().map_err(strict_encoding::Error::from)?;
        Ok(())
    }
}

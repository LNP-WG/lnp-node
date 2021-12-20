// LNP Node: node running lightning network protocol and generalized lightning
// channels.
// Written in 2020 by
//     Dr. Maxim Orlovsky <orlovsky@pandoracore.com>
//
// To the extent possible under law, the author(s) have dedicated all
// copyright and related and neighboring rights to this software to
// the public domain worldwide. This software is distributed without
// any warranty.
//
// You should have received a copy of the MIT License
// along with this software.
// If not, see <https://opensource.org/licenses/MIT>.

use std::time::SystemTime;

use amplify::{DumbDefault, Slice32};
use bitcoin::hashes::Hash;
use internet2::NodeAddr;
use lnp::bolt;
use lnp::bolt::{CommonParams, LocalKeyset, PeerParams, Policy};
use lnp::channel::Channel;
use lnp::p2p::legacy::{Messages as LnMsg, TempChannelId};
use microservices::esb::{self, Handler};

use super::storage::{self, Driver};
use crate::bus::{self, BusMsg, CtlMsg, ServiceBus};
use crate::channeld::state_machines::ChannelStateMachine;
use crate::rpc::{ClientId, ServiceId};
use crate::{Config, CtlServer, Endpoints, Error, Service};

pub fn run(config: Config, temp_channel_id: TempChannelId) -> Result<(), Error> {
    let chain_hash = config.chain.as_genesis_hash().as_inner();
    // TODO: use node configuration to provide custom policy & parameters
    let channel = Channel::with(
        temp_channel_id,
        Slice32::from(chain_hash),
        Policy::default(),
        CommonParams::default(),
        PeerParams::default(),
        LocalKeyset::dumb_default(), // we do not have keyset derived at this stage
    );

    let runtime = Runtime {
        identity: ServiceId::Channel(temp_channel_id.into()),
        peer_service: ServiceId::Loopback,
        state_machine: default!(),
        channel,
        remote_peer: None,
        started: SystemTime::now(),
        obscuring_factor: 0,
        enquirer: None,
        storage: Box::new(storage::DiskDriver::init(
            temp_channel_id.into(),
            Box::new(storage::DiskConfig { path: Default::default() }),
        )?),
    };

    Service::run(config, runtime, false)
}

pub struct Runtime {
    identity: ServiceId,
    pub(super) peer_service: ServiceId,
    pub(super) state_machine: ChannelStateMachine,
    pub(super) channel: Channel<bolt::ExtensionId>,
    remote_peer: Option<NodeAddr>,

    // From here till the `enqueror` all parameters should be removed since they are part of
    // `channel` now
    started: SystemTime,
    obscuring_factor: u64,

    // TODO: Refactor to use ClientId
    enquirer: Option<ClientId>,

    storage: Box<dyn storage::Driver>,
}

impl CtlServer for Runtime {
    #[inline]
    fn enquirer(&self) -> Option<ClientId> { self.enquirer.clone() }
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
            (ServiceBus::Msg, BusMsg::Ln(msg), ServiceId::Peer(remote_peer)) => {
                self.handle_p2p(endpoints, remote_peer, msg)
            }
            (ServiceBus::Msg, BusMsg::Ln(_), service) => {
                unreachable!("channeld received peer message not from a peerd but from {}", service)
            }
            (ServiceBus::Ctl, BusMsg::Ctl(msg), source) => self.handle_ctl(endpoints, source, msg),
            (ServiceBus::Rpc, ..) => unreachable!("peer daemon must not bind to RPC interface"),
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
        identity: ServiceId,
    ) -> Result<(), Error> {
        endpoints.set_identity(ServiceBus::Ctl, identity.clone())?;
        endpoints.set_identity(ServiceBus::Msg, identity.clone())?;
        self.identity = identity;
        Ok(())
    }

    #[cfg(feature = "rgb")]
    fn request_rbg20(
        &mut self,
        request: rgb_node::rpc::fungible::Request,
    ) -> Result<rgb_node::rpc::Reply, Error> {
        let data = request.serialize();
        self.rgb20_rpc.send_raw_message(&data)?;
        let raw = self.rgb20_rpc.recv_raw_message()?;
        let reply = &*self.rgb_unmarshaller.unmarshall(&raw)?;
        if let rgb_node::rpc::Reply::Failure(failure) = reply {
            error!("{} {}", "RGB Node reported failure:".err(), failure.err())
        }
        Ok(reply.clone())
    }

    pub fn send_p2p(
        &self,
        endpoints: &mut Endpoints,
        message: LnMsg,
    ) -> Result<(), esb::Error<ServiceId>> {
        endpoints.send_to(
            ServiceBus::Msg,
            self.identity(),
            self.peer_service.clone(),
            BusMsg::Ln(message),
        )?;
        Ok(())
    }

    fn handle_p2p(
        &mut self,
        endpoints: &mut Endpoints,
        remote_peer: NodeAddr,
        message: LnMsg,
    ) -> Result<(), Error> {
        match message {
            LnMsg::OpenChannel(_) => {
                // TODO: Support repeated messages according to BOLT-2 requirements
                // if the connection has been re-established after receiving a previous
                // open_channel, BUT before receiving a funding_created message:
                //     accept a new open_channel message.
                //     discard the previous open_channel message.
                warn!(
                    "Got `open_channel` P2P message from {}, which is unexpected: the channel \
                     creation was already requested before",
                    remote_peer
                );
            }

            LnMsg::AcceptChannel(_) | LnMsg::FundingSigned(_) | LnMsg::FundingLocked(_) => {
                self.process(endpoints, ServiceId::Peer(remote_peer), BusMsg::Ln(message))?;
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
        match request.clone() {
            // Proposing remote peer to open a channel
            CtlMsg::OpenChannelWith(ref open_channel_with) => {
                let remote_peer = open_channel_with.remote_peer.clone();
                self.enquirer = open_channel_with.report_to.clone();
                // Updating state only if the request was processed
                self.peer_service = ServiceId::Peer(remote_peer.clone());
                self.remote_peer = Some(remote_peer);
                self.process(endpoints, source, BusMsg::Ctl(request))?;
            }

            // Processing remote request to open a channel
            CtlMsg::AcceptChannelFrom(bus::AcceptChannelFrom { ref remote_peer, .. }) => {
                self.enquirer = None;
                let remote_peer = remote_peer.clone();
                if self.process(endpoints, source, BusMsg::Ctl(request))? {
                    // Updating state only if the request was processed
                    self.peer_service = ServiceId::Peer(remote_peer.clone());
                    self.remote_peer = Some(remote_peer);
                }
            }

            CtlMsg::FundingConstructed(_)
            | CtlMsg::FundingPublished
            | CtlMsg::Mined(_)
            | CtlMsg::Signed(_)
            | CtlMsg::Error { .. }
            | CtlMsg::EsbError { .. } => {
                self.process(endpoints, source, BusMsg::Ctl(request))?;
            }

            _ => {
                error!("Request is not supported by the CTL interface");
                return Err(Error::wrong_esb_msg(ServiceBus::Ctl, &request));
            }
        }
        Ok(())
    }
}

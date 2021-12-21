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

use internet2::NodeAddr;
use lnp::p2p::legacy::{Messages as LnMsg, TempChannelId};
use microservices::esb::{self, Handler};

use super::storage::{self, Driver};
use super::ChannelState;
use crate::bus::{self, BusMsg, CtlMsg, ServiceBus};
use crate::rpc::{ClientId, ServiceId};
use crate::{Config, CtlServer, Endpoints, Error, Service};

pub fn run(config: Config, temp_channel_id: TempChannelId) -> Result<(), Error> {
    // TODO: use node configuration to provide custom policy & parameters

    let runtime = Runtime {
        identity: ServiceId::Channel(temp_channel_id.into()),
        state: ChannelState::with(temp_channel_id, &config.chain),
        started: SystemTime::now(),
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
    pub(super) state: ChannelState,
    started: SystemTime,
    /// Client which is made an equiry starting the current workflow run by the active state
    /// machine. It is not a part of the state of the machine since it should not persist.
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

    pub fn send_p2p(
        &self,
        endpoints: &mut Endpoints,
        message: LnMsg,
    ) -> Result<(), esb::Error<ServiceId>> {
        let remote_peer = self.state.remote_peer.clone().expect("unset remote peer in channeld");
        endpoints.send_to(
            ServiceBus::Msg,
            self.identity(),
            ServiceId::Peer(remote_peer),
            BusMsg::Ln(message),
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

            LnMsg::ChannelReestablish(_) => {
                // TODO: Consider moving setting remote peer and equirer to the state machines
                self.enquirer = None;
                let remote_peer = remote_peer.clone();
                let peerd = ServiceId::Peer(remote_peer.clone());
                if self.process(endpoints, peerd, BusMsg::Ln(message))? {
                    // Updating state only if the request was processed
                    self.state.remote_peer = Some(remote_peer);
                }
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

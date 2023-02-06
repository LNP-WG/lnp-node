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

pub mod accept;
pub mod propose;

use bitcoin::secp256k1;
use bitcoin::secp256k1::PublicKey;
use lnp::channel;
use lnp::channel::bolt::Lifecycle;
use lnp::p2p::bolt::{ActiveChannelId, ChannelReestablish, Messages as LnMsg};
use lnp_rpc::FailureCode;
use microservices::cli::LogStyle;
use microservices::esb;
use microservices::esb::Handler;
use strict_encoding::StrictEncode;

use self::accept::ChannelAccept;
use self::propose::ChannelPropose;
use crate::automata::{Event, StateMachine};
use crate::bus::{BusMsg, CtlMsg};
use crate::channeld::runtime::Runtime;
use crate::rpc::{Failure, ServiceId};
use crate::{Endpoints, Responder};

/// Errors for channel proposal workflow
#[derive(Clone, Debug, Display, From, Error)]
#[display(doc_comments)]
pub enum Error {
    /// unexpected message from {2} for a channel state {1}. Message details: {0}
    UnexpectedMessage(BusMsg, Lifecycle, ServiceId),

    /// generic LNP channel error
    #[from]
    #[display(inner)]
    Channel(lnp::channel::bolt::Error),

    /// error sending RPC request during state transition. Details: {0}
    #[from]
    Esb(esb::Error<ServiceId>),

    /// unable to {operation} during {current_state} channel state
    InvalidState { operation: &'static str, current_state: Lifecycle },

    /// channel was not persisted on a disk, so unable to reestablish
    NoPersistantData,

    /// sign daemon was unable to sign funding transaction for our public key {0}
    FundingPsbtUnsigned(PublicKey),

    /// sign daemon produced invalid signature. {0}
    InvalidSig(secp256k1::Error),

    /// failed to save channel state. Details: {0}
    #[from]
    Persistence(strict_encoding::Error),
}

impl Error {
    /// Returns unique error number sent to the client alongside text message to help run
    /// client-side diagnostics
    pub fn errno(&self) -> u16 {
        match self {
            Error::UnexpectedMessage(_, _, _) => 1001,
            Error::Channel(channel::bolt::Error::ChannelReestablish(_)) => 2001,
            Error::Channel(channel::bolt::Error::Htlc(_)) => 2002,
            Error::Channel(channel::bolt::Error::Policy(_)) => 2003,
            Error::Channel(channel::bolt::Error::LifecycleMismatch { .. }) => 2004,
            Error::Channel(channel::bolt::Error::Funding(_)) => 2005,
            Error::Channel(channel::bolt::Error::Route(_)) => 2006,
            Error::Channel(channel::bolt::Error::NoChanelId) => 2007,
            Error::Channel(channel::bolt::Error::NoTemporaryId) => 2008,
            Error::Esb(_) => 3001,
            Error::InvalidState { .. } => 4001,
            Error::FundingPsbtUnsigned(_) => 5001,
            Error::InvalidSig(_) => 5002,
            Error::Persistence(_) => 6000,
            Error::NoPersistantData => 6001,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Display, From)]
#[derive(StrictEncode, StrictDecode)]
pub enum ChannelStateMachine {
    /// launching channel daemon
    #[display("LAUNCH")]
    Launch,

    /// proposing remote peer to open channel
    #[display(inner)]
    #[from]
    Propose(ChannelPropose),

    /// accepting channel proposed by a remote peer
    #[display("ACCEPT")]
    #[from]
    Accept(ChannelAccept),

    /// active channel operations
    #[display("ACTIVE")]
    Active,

    /// reestablishing channel
    #[display("REESTABLISHING")]
    Reestablishing,

    /// cooperatively closing channel
    #[display("CLOSING")]
    Closing,

    /// uncooperative channel closing initiated by thyself
    #[display("ABORT")]
    Abort,

    /// reacting to an uncooperative channel close from remote
    #[display("PENALIZE")]
    Penalize,
}

// TODO: Replace with method checking persistence data on the disk and initializing state machine
//       according to them
impl Default for ChannelStateMachine {
    #[inline]
    fn default() -> Self { ChannelStateMachine::Launch }
}

impl ChannelStateMachine {
    /// Computes channel lifecycle stage for the current channel proposal workflow stage
    pub fn lifecycle(&self) -> Lifecycle {
        match self {
            ChannelStateMachine::Launch => Lifecycle::Initial,
            ChannelStateMachine::Propose(state_machine) => state_machine.lifecycle(),
            ChannelStateMachine::Accept(state_machine) => state_machine.lifecycle(),
            ChannelStateMachine::Active => Lifecycle::Active,
            ChannelStateMachine::Reestablishing => Lifecycle::Reestablishing,
            // TODO: use state machine
            ChannelStateMachine::Closing => Lifecycle::Closing { round: 0 },
            ChannelStateMachine::Abort => Lifecycle::Aborting,
            ChannelStateMachine::Penalize => Lifecycle::Penalize,
        }
    }

    pub(self) fn info_message(&self, channel_id: ActiveChannelId) -> String {
        match self {
            ChannelStateMachine::Launch => s!("Launching channel daemon"),
            ChannelStateMachine::Propose(state_machine) => state_machine.info_message(channel_id),
            ChannelStateMachine::Accept(state_machine) => state_machine.info_message(channel_id),
            ChannelStateMachine::Active => s!("Channel is active"),
            ChannelStateMachine::Reestablishing => s!("Reestablishing channel"),
            ChannelStateMachine::Closing => s!("Closing channel"),
            ChannelStateMachine::Abort => s!("Unilaterally closing the channel"),
            ChannelStateMachine::Penalize => s!("Penalizing incorrect channel"),
        }
    }
}

impl Runtime {
    /// Processes incoming RPC or peer requests updating state - and switching to a new state, if
    /// necessary. Returns bool indicating whether a successful state update happened
    pub fn process(
        &mut self,
        endpoints: &mut Endpoints,
        source: ServiceId,
        request: BusMsg,
    ) -> Result<bool, Error> {
        if let BusMsg::Ctl(CtlMsg::EsbError { destination, error: _ }) = &request {
            let (code, info) = match destination {
                ServiceId::PeerBolt(remote_peer) => (
                    FailureCode::Channel,
                    format!(
                        "There is no connection with the remote peer {}; you have to `connect` to \
                         it first",
                        remote_peer
                    ),
                ),
                _ => (
                    FailureCode::Channel,
                    format!("Unable to complete: daemon {} is offline or crashed", destination),
                ),
            };
            self.report_failure(endpoints, Failure { code, info });
        }

        let event = Event::with(endpoints, self.identity(), source, request);
        let channel_id = self.state.channel.active_channel_id();
        let updated_state = match self.process_event(event) {
            Ok(_) => {
                // Ignoring possible reporting errors here and after: do not want to
                // halt the channel just because the client disconnected
                let _ = self
                    .report_progress(endpoints, self.state.state_machine.info_message(channel_id));
                true
            }
            // We pass ESB errors forward such that they can fail the channel.
            // In the future they can be caught here and used to re-iterate sending of the same
            // message later without channel halting.
            Err(err @ Error::Esb(_)) => {
                error!("{} due to ESB failure: {}", "Failing channel".err(), err.err_details());
                self.report_failure(endpoints, Failure {
                    code: FailureCode::Channel,
                    info: err.to_string(),
                });
                return Err(err);
            }
            Err(other_err) => {
                error!("{}: {}", "Channel error".err(), other_err.err_details());
                self.report_failure(endpoints, Failure {
                    code: FailureCode::Channel,
                    info: other_err.to_string(),
                });
                false
            }
        };
        if updated_state {
            self.save_state()?;
            info!(
                "ChannelStateMachine {} switched to {} state",
                self.state.channel.active_channel_id(),
                self.state.state_machine
            );
        }
        Ok(updated_state)
    }

    fn process_event(&mut self, event: Event<BusMsg>) -> Result<(), Error> {
        // We have to handle channel reestablishment separately, since this is
        // shared across multiple channel states
        if let BusMsg::Bolt(LnMsg::ChannelReestablish(ref remote_channel_reestablish)) =
            event.message
        {
            self.state.state_machine = self.complete_reestablish(
                event.endpoints,
                event.source,
                remote_channel_reestablish,
            )?;
            return Ok(());
        }

        self.state.state_machine = match self.state.state_machine {
            ChannelStateMachine::Launch => self.complete_launch(event),
            ChannelStateMachine::Propose(channel_propose) => {
                self.process_propose(event, channel_propose)
            }
            ChannelStateMachine::Accept(channel_accept) => {
                self.process_accept(event, channel_accept)
            }
            ChannelStateMachine::Active => Ok(ChannelStateMachine::Active), // TODO
            // This is when we were launched by lnpd with a aim of re-establishing channel;
            // the state is valid _before_ we receive channel_reestablish from the peer.
            ChannelStateMachine::Reestablishing => todo!(),
            ChannelStateMachine::Closing => todo!(),
            ChannelStateMachine::Abort => todo!(),
            ChannelStateMachine::Penalize => todo!(),
        }?;
        Ok(())
    }

    fn complete_reestablish(
        &mut self,
        endpoints: &mut Endpoints,
        source: ServiceId,
        remote_channel_reestablish: &ChannelReestablish,
    ) -> Result<ChannelStateMachine, Error> {
        let local_channel_reestablish =
            self.state.channel.compose_reestablish_channel(remote_channel_reestablish)?;
        let remote_id =
            source.to_remote_id().expect("channel reestablish BOLT message from non-remoter peer");
        self.state.remote_id = Some(remote_id);
        self.send_p2p(endpoints, LnMsg::ChannelReestablish(local_channel_reestablish))?;

        // We swallow error since we do not want to fail the channel if we just can't add it to the
        // router
        trace!("Notifying remote peer about channel reestablishing");
        let remote_id = self.state.remote_id();
        let message = CtlMsg::ChannelCreated(self.state.channel.channel_info(remote_id));
        let _ = self.send_ctl(endpoints, ServiceId::Router, message);

        Ok(ChannelStateMachine::Active)
    }

    fn complete_launch(&mut self, event: Event<BusMsg>) -> Result<ChannelStateMachine, Error> {
        let Event { endpoints, service: _, source, message } = event;
        Ok(match message {
            BusMsg::Ctl(CtlMsg::OpenChannelWith(open_channel_with)) => {
                ChannelPropose::with(self, endpoints, open_channel_with)?.into()
            }
            BusMsg::Ctl(CtlMsg::AcceptChannelFrom(accept_channel_from)) => {
                ChannelAccept::with(self, endpoints, accept_channel_from)?.into()
            }
            wrong_msg => {
                return Err(Error::UnexpectedMessage(wrong_msg, Lifecycle::Initial, source))
            }
        })
    }

    fn process_propose(
        &mut self,
        event: Event<BusMsg>,
        channel_propose: ChannelPropose,
    ) -> Result<ChannelStateMachine, Error> {
        Ok(match channel_propose.next(event, self)? {
            None => ChannelStateMachine::Active,
            Some(channel_propose) => ChannelStateMachine::Propose(channel_propose),
        })
    }

    fn process_accept(
        &mut self,
        event: Event<BusMsg>,
        channel_accept: ChannelAccept,
    ) -> Result<ChannelStateMachine, Error> {
        Ok(match channel_accept.next(event, self)? {
            None => ChannelStateMachine::Active,
            Some(channel_accept) => ChannelStateMachine::Accept(channel_accept),
        })
    }
}

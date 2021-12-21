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

pub mod accept;
pub mod propose;

use bitcoin::secp256k1;
use bitcoin::secp256k1::PublicKey;
use lnp::bolt::Lifecycle;
use lnp::p2p::legacy::{ActiveChannelId, Messages as LnMsg};
use microservices::esb;
use microservices::esb::Handler;

use self::accept::ChannelAccept;
use self::propose::ChannelPropose;
use crate::automata::{Event, StateMachine};
use crate::bus::{BusMsg, CtlMsg};
use crate::channeld::runtime::Runtime;
use crate::rpc::{Failure, ServiceId};
use crate::service::LogStyle;
use crate::{CtlServer, Endpoints};

/// Errors for channel proposal workflow
#[derive(Clone, Debug, Display, From, Error)]
#[display(doc_comments)]
pub enum Error {
    /// unexpected message from {2} for a channel state {1}. Message details: {0}
    UnexpectedMessage(BusMsg, Lifecycle, ServiceId),

    /// generic LNP channel error
    #[from]
    #[display(inner)]
    Channel(lnp::channel::Error),

    /// error sending RPC request during state transition. Details: {0}
    #[from]
    Esb(esb::Error<ServiceId>),

    /// unable to {operation} during {current_state} channel state
    InvalidState { operation: &'static str, current_state: Lifecycle },

    /// sign daemon was unable to sign funding transaction for our public key {0}
    FundingPsbtUnsigned(PublicKey),

    /// sign daemon produced invalid signature. {0}
    InvalidSig(secp256k1::Error),
}

impl Error {
    /// Returns unique error number sent to the client alongside text message to help run
    /// client-side diagnostics
    pub fn errno(&self) -> u16 {
        match self {
            Error::UnexpectedMessage(_, _, _) => 1001,
            Error::Channel(lnp::channel::Error::Extension(_)) => 2001,
            Error::Channel(lnp::channel::Error::Htlc(_)) => 2002,
            Error::Channel(lnp::channel::Error::Policy(_)) => 2003,
            Error::Channel(lnp::channel::Error::LifecycleMismatch { .. }) => 2004,
            Error::Channel(lnp::channel::Error::Funding(_)) => 2005,
            Error::Esb(_) => 3001,
            Error::InvalidState { .. } => 4001,
            Error::FundingPsbtUnsigned(_) => 5001,
            Error::InvalidSig(_) => 5002,
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
            ChannelStateMachine::Active => todo!(),
            ChannelStateMachine::Reestablishing => todo!("Process in reestablishing state machine"),
            ChannelStateMachine::Closing => todo!(),
            ChannelStateMachine::Abort => todo!(),
            ChannelStateMachine::Penalize => todo!(),
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
                ServiceId::Peer(remote_peer) => (
                    9001,
                    format!(
                        "There is no connection with the remote peer {}; you have to `connect` to \
                         it first",
                        remote_peer
                    ),
                ),
                _ => (
                    9000,
                    format!("Unable to complete: daemon {} is offline or crashed", destination),
                ),
            };
            self.report_failure(endpoints, Failure { code, info });
        }

        let event = Event::with(endpoints, self.identity(), source, request);
        let channel_id = self.channel.active_channel_id();
        let updated_state = match self.process_event(event) {
            Ok(_) => {
                // Ignoring possible reporting errors here and after: do not want to
                // halt the channel just because the client disconnected
                let _ =
                    self.report_progress(endpoints, self.state_machine.info_message(channel_id));
                true
            }
            // We pass ESB errors forward such that they can fail the channel.
            // In the future they can be caught here and used to re-iterate sending of the same
            // message later without channel halting.
            Err(err @ Error::Esb(_)) => {
                error!("{} due to ESB failure: {}", "Failing channel".err(), err.err_details());
                self.report_failure(endpoints, Failure {
                    code: err.errno(),
                    info: err.to_string(),
                });
                return Err(err);
            }
            Err(other_err) => {
                error!("{}: {}", "Channel error".err(), other_err.err_details());
                self.report_failure(endpoints, Failure {
                    code: other_err.errno(),
                    info: other_err.to_string(),
                });
                false
            }
        };
        if updated_state {
            info!(
                "ChannelStateMachine {} switched to {} state",
                self.channel.active_channel_id(),
                self.state_machine
            );
        }
        Ok(updated_state)
    }

    fn process_event(&mut self, event: Event<BusMsg>) -> Result<(), Error> {
        self.state_machine = match self.state_machine {
            ChannelStateMachine::Launch => self.complete_launch(event),
            ChannelStateMachine::Propose(channel_propose) => {
                self.process_propose(event, channel_propose)
            }
            ChannelStateMachine::Accept(channel_accept) => {
                self.process_accept(event, channel_accept)
            }
            ChannelStateMachine::Active => Ok(ChannelStateMachine::Active), // TODO
            ChannelStateMachine::Reestablishing => todo!(),
            ChannelStateMachine::Closing => todo!(),
            ChannelStateMachine::Abort => todo!(),
            ChannelStateMachine::Penalize => todo!(),
        }?;
        Ok(())
    }

    fn complete_launch(&mut self, event: Event<BusMsg>) -> Result<ChannelStateMachine, Error> {
        let Event { endpoints, service: _, source, message } = event;
        Ok(match message {
            BusMsg::Ctl(CtlMsg::OpenChannelWith(open_channel_with)) => {
                ChannelPropose::with(self, endpoints, open_channel_with)?.into()
            }
            BusMsg::Ln(LnMsg::ChannelReestablish(_)) => {
                // TODO: Initialize reestablishing state machine
                ChannelStateMachine::Reestablishing
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

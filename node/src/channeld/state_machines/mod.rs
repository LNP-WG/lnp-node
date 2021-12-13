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

pub mod channel_accept;
pub mod channel_propose;

use lnp::bolt::Lifecycle;
use lnp::p2p::legacy::ActiveChannelId;
use microservices::esb;
use microservices::esb::Handler;

use self::channel_propose::ChannelPropose;
use crate::channeld::runtime::Runtime;
use crate::i9n::ctl::{CtlMsg, OpenChannelWith};
use crate::i9n::rpc::Failure;
use crate::i9n::BusMsg;
use crate::service::LogStyle;
use crate::state_machine::Event;
use crate::{CtlServer, Endpoints, ServiceId};

/// Errors for channel proposal workflow
#[derive(Clone, Debug, Display, From, Error)]
#[display(doc_comments)]
pub enum Error {
    /// unexpected message for a channel state {1}. Message details: {0}
    UnexpectedMessage(CtlMsg, Lifecycle),

    /// generic LNP channel error
    #[from]
    #[display(inner)]
    Channel(lnp::channel::Error),

    /// error sending RPC request during state transition. Details: {0}
    #[from]
    Esb(esb::Error),

    /// unable to {operation} during {current_state} channel state
    InvalidState { operation: &'static str, current_state: Lifecycle },
}

impl Error {
    /// Returns unique error number sent to the client alongside text message to help run
    /// client-side diagnostics
    pub fn errno(&self) -> u16 {
        match self {
            Error::UnexpectedMessage(_, _) => 1001,
            Error::Channel(lnp::channel::Error::Extension(_)) => 2001,
            Error::Channel(lnp::channel::Error::Htlc(_)) => 2002,
            Error::Channel(lnp::channel::Error::Policy(_)) => 2003,
            Error::Channel(lnp::channel::Error::LifecycleMismatch { .. }) => 2004,
            Error::Esb(_) => 3001,
            Error::InvalidState { .. } => 4001,
        }
    }
}

#[derive(Debug, Display)]
pub enum ChannelStateMachine {
    /// launching channel daemon
    #[display("LAUNCH")]
    Launch,

    /// proposing remote peer to open channel
    #[display(inner)]
    Propose(channel_propose::ChannelPropose),

    /// accepting channel proposed by a remote peer
    #[display("ACCEPT")]
    Accept(channel_accept::ChannelAccept),

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
            ChannelStateMachine::Reestablishing => todo!(),
            ChannelStateMachine::Closing => todo!(),
            ChannelStateMachine::Abort => todo!(),
            ChannelStateMachine::Penalize => todo!(),
        }
    }
}

impl Runtime {
    pub fn propose_channel(
        &mut self,
        endpoints: &mut Endpoints,
        request: OpenChannelWith,
    ) -> Result<(), Error> {
        if !matches!(self.state_machine, ChannelStateMachine::Launch) {
            return Err(Error::InvalidState {
                operation: "open channel",
                current_state: self.state_machine.lifecycle(),
            });
        }
        self.state_machine =
            ChannelStateMachine::Propose(ChannelPropose::with(self, endpoints, request)?);
        Ok(())
    }

    /// Processes incoming RPC or peer requests updating state - and switching to a new state, if
    /// necessary. Returns bool indicating whether a successful state update happened
    pub fn process(
        &mut self,
        endpoints: &mut Endpoints,
        source: ServiceId,
        request: BusMsg,
    ) -> Result<bool, Error> {
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

    fn process_event(&mut self, _event: Event<BusMsg>) -> Result<(), Error> {
        match self.state_machine {
            _ => {} // TODO: implement
        }
        Ok(())
    }
}

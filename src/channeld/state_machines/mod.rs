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
use microservices::esb::Handler;

use self::channel_propose::ChannelPropose;
use crate::channeld::runtime::Runtime;
use crate::state_machine::Event;
use crate::{rpc, Senders, ServiceId};

/// Errors for channel state machine
#[derive(Debug, Display, From, Error)]
#[display(inner)]
pub enum Error {
    /// Error during channel propose workflow
    #[from]
    ChannelPropose(channel_propose::Error),
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
    Accept,

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
            ChannelStateMachine::Accept => Lifecycle::Accepted, // TODO: use state machine,
            ChannelStateMachine::Active => Lifecycle::Active,
            ChannelStateMachine::Reestablishing => Lifecycle::Reestablishing,
            // TODO: use state machine
            ChannelStateMachine::Closing => Lifecycle::Closing { round: 0 },
            ChannelStateMachine::Abort => Lifecycle::Aborting,
            ChannelStateMachine::Penalize => Lifecycle::Penalize,
        }
    }
}

impl Runtime {
    /// Processes incoming RPC or peer requests updating state - and switching to a new state, if
    /// necessary. Returns bool indicating whether state switch had happened
    pub fn process(
        &mut self,
        senders: &mut Senders,
        source: ServiceId,
        request: rpc::Request,
    ) -> Result<bool, Error> {
        let event = Event::with(senders, self.identity(), source, request);
        let switched_state = match self.state_machine {
            ChannelStateMachine::Launch => {
                self.state_machine =
                    ChannelStateMachine::Propose(ChannelPropose::with(event, self)?);
                true
            }
            _ => false, // TODO: implement
        };
        if switched_state {
            info!(
                "ChannelStateMachine {} switched to {} state",
                self.channel.active_channel_id(),
                self.state_machine
            );
        }
        Ok(switched_state)
    }
}

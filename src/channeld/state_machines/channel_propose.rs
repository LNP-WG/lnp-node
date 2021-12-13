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

use lnp::bolt::Lifecycle;
use lnp::p2p::legacy::{ActiveChannelId, Messages};

use crate::channeld::runtime::Runtime;
use crate::channeld::state_machines;
use crate::i9n::ctl::{CtlMsg, OpenChannelWith};
use crate::service::LogStyle;
use crate::state_machine::{Event, StateMachine};
use crate::Endpoints;

/// Channel proposal workflow
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug, Display)]
pub enum ChannelPropose {
    /// asked remote peer to accept a new channel
    #[display("PROPOSED")]
    Proposed,

    /// remote peer accepted our channel proposal
    #[display("ACCEPTED")]
    Accepted,

    /// sent funding txid and commitment signature to the remote peer
    #[display("FUNDING")]
    Funding,

    /// received signed commitment from the remote peer
    #[display("SIGNED")]
    Signed,

    /// awaiting funding transaction to be mined
    #[display("FUNDED")]
    Funded,

    /// funding transaction is mined, awaiting for the other peer confirmation of this fact
    #[display("LOCKED")]
    Locked,
}

impl StateMachine<CtlMsg, Runtime> for ChannelPropose {
    type Error = state_machines::Error;

    fn next(
        self,
        event: Event<CtlMsg>,
        runtime: &mut Runtime,
    ) -> Result<Option<Self>, Self::Error> {
        let channel_id = runtime.channel.active_channel_id();
        debug!("ChannelPropose {} received {} event", channel_id, event.message);
        let state = match self {
            ChannelPropose::Proposed => finish_proposed(event, runtime),
            ChannelPropose::Accepted => finish_accepted(event, runtime),
            ChannelPropose::Funding => finish_funding(event, runtime),
            ChannelPropose::Signed => finish_signed(event, runtime),
            ChannelPropose::Funded => finish_funded(event, runtime),
            ChannelPropose::Locked => {
                finish_locked(event, runtime)?;
                info!("ChannelPropose {} has completed its work", channel_id);
                return Ok(None);
            }
        }?;
        info!("ChannelPropose {} switched to {} state", channel_id, state);
        Ok(Some(state))
    }
}

impl ChannelPropose {
    /// Computes channel lifecycle stage for the current channel proposal workflow stage
    pub fn lifecycle(&self) -> Lifecycle {
        match self {
            ChannelPropose::Proposed => Lifecycle::Proposed,
            ChannelPropose::Accepted => Lifecycle::Accepted,
            ChannelPropose::Funding => Lifecycle::Funding,
            ChannelPropose::Signed => Lifecycle::Signed,
            ChannelPropose::Funded => Lifecycle::Funded,
            ChannelPropose::Locked => Lifecycle::Locked,
        }
    }
}

// State transitions:

impl ChannelPropose {
    /// Constructs channel proposal state machine
    pub fn with(
        runtime: &mut Runtime,
        senders: &mut Endpoints,
        request: OpenChannelWith,
    ) -> Result<ChannelPropose, state_machines::Error> {
        let open_channel = Messages::OpenChannel(
            runtime.channel.open_channel_compose(request.funding_sat, request.push_msat)?,
        );

        runtime.send_p2p(senders, open_channel)?;

        Ok(ChannelPropose::Proposed)
    }

    /// Construct information message for error and client reporting
    pub fn info_message(&self, channel_id: ActiveChannelId) -> String {
        match self {
            ChannelPropose::Proposed => {
                format!(
                    "{} remote peer to {} with temp id {:#}",
                    "Proposing".promo(),
                    "open a channel".promo(),
                    channel_id.promoter()
                )
            }
            _ => todo!(),
        }
    }
}

fn finish_proposed(
    _event: Event<CtlMsg>,
    _runtime: &mut Runtime,
) -> Result<ChannelPropose, state_machines::Error> {
    todo!()
}

fn finish_accepted(
    _event: Event<CtlMsg>,
    _runtime: &mut Runtime,
) -> Result<ChannelPropose, state_machines::Error> {
    todo!()
}

fn finish_funding(
    _event: Event<CtlMsg>,
    _runtime: &mut Runtime,
) -> Result<ChannelPropose, state_machines::Error> {
    todo!()
}

fn finish_signed(
    _event: Event<CtlMsg>,
    _runtime: &mut Runtime,
) -> Result<ChannelPropose, state_machines::Error> {
    todo!()
}

fn finish_funded(
    _event: Event<CtlMsg>,
    _runtime: &mut Runtime,
) -> Result<ChannelPropose, state_machines::Error> {
    todo!()
}

fn finish_locked(
    _event: Event<CtlMsg>,
    _runtime: &mut Runtime,
) -> Result<(), state_machines::Error> {
    todo!()
}

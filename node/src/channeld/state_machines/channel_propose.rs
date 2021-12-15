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
use lnp::p2p::legacy::{ActiveChannelId, FundingCreated, Messages as LnMsg};

use super::Error;
use crate::channeld::runtime::Runtime;
use crate::channeld::state_machines;
use crate::i9n::ctl::{CtlMsg, FundChannel, OpenChannelWith};
use crate::i9n::BusMsg;
use crate::service::LogStyle;
use crate::state_machine::{Event, StateMachine};
use crate::{CtlServer, Endpoints, ServiceId};

/// Channel proposal workflow
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug, Display)]
pub enum ChannelPropose {
    /// asked remote peer to accept a new channel
    #[display("PROPOSED")]
    Proposed,

    /// remote peer accepted our channel proposal
    #[display("ACCEPTED")]
    Accepted,

    /// signing refund transaction on our side
    #[display("SIGNING")]
    Signing,

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

impl StateMachine<BusMsg, Runtime> for ChannelPropose {
    type Error = state_machines::Error;

    fn next(
        self,
        event: Event<BusMsg>,
        runtime: &mut Runtime,
    ) -> Result<Option<Self>, Self::Error> {
        let channel_id = runtime.channel.active_channel_id();
        debug!("ChannelPropose {} received {} event", channel_id, event.message);
        let state = match self {
            ChannelPropose::Proposed => complete_proposed(event, runtime),
            ChannelPropose::Accepted => complete_accepted(event, runtime),
            ChannelPropose::Signing => complete_signing(event, runtime),
            ChannelPropose::Funding => complete_funding(event, runtime),
            ChannelPropose::Signed => complete_signed(event, runtime),
            ChannelPropose::Funded => complete_funded(event, runtime),
            ChannelPropose::Locked => {
                complete_locked(event, runtime)?;
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
            ChannelPropose::Signing => Lifecycle::Signing,
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
        endpoints: &mut Endpoints,
        request: OpenChannelWith,
    ) -> Result<ChannelPropose, state_machines::Error> {
        let open_channel = LnMsg::OpenChannel(
            runtime.channel.compose_open_channel(request.funding_sat, request.push_msat)?,
        );

        runtime.send_p2p(endpoints, open_channel)?;

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

fn complete_proposed(
    event: Event<BusMsg>,
    runtime: &mut Runtime,
) -> Result<ChannelPropose, state_machines::Error> {
    let accept_channel = match event.message {
        BusMsg::Ln(LnMsg::AcceptChannel(accept_channel)) => accept_channel,
        wrong_msg => {
            return Err(Error::UnexpectedMessage(wrong_msg, Lifecycle::Proposed, event.source))
        }
    };

    let channel = &mut runtime.channel;
    channel.update_from_accept_channel(accept_channel)?;
    let fund_channel = FundChannel {
        address: channel.funding_address(),
        fee: channel.funding_fee(),
        amount: channel.local_amount(),
    };

    runtime.send_ctl(event.endpoints, ServiceId::Lnpd, CtlMsg::ConstructFunding(fund_channel))?;
    Ok(ChannelPropose::Accepted)
}

fn complete_accepted(
    event: Event<BusMsg>,
    runtime: &mut Runtime,
) -> Result<ChannelPropose, state_machines::Error> {
    let funding_outpoint = match event.message {
        BusMsg::Ctl(CtlMsg::FundingConstructed(funding_outpoint)) => funding_outpoint,
        wrong_msg => {
            return Err(Error::UnexpectedMessage(wrong_msg, Lifecycle::Accepted, event.source))
        }
    };

    let channel = &mut runtime.channel;
    let refund_psbt = channel.construct_refund(funding_outpoint);

    runtime.send_ctl(event.endpoints, ServiceId::Signer, CtlMsg::Sign(refund_psbt))?;
    Ok(ChannelPropose::Signing)
}

fn complete_signing(
    event: Event<BusMsg>,
    runtime: &mut Runtime,
) -> Result<ChannelPropose, state_machines::Error> {
    let refund_psbt = match event.message {
        BusMsg::Ctl(CtlMsg::Signed(psbt)) => psbt,
        // TODO: Change lifecycle to Lifecycle::Signing
        wrong_msg => {
            return Err(Error::UnexpectedMessage(wrong_msg, Lifecycle::Accepted, event.source))
        }
    };

    let channel = &runtime.channel;
    let funding_created = FundingCreated {
        temporary_channel_id: channel
            .temp_channel_id()
            .expect("channel at funding stage must have temporary channel id"),
        funding_txid: channel.funding_outpoint().txid,
        funding_output_index: channel.funding_outpoint().vout as u16,
        // TODO: Extract signature
        signature: todo!("refund_psbt.inputs[0].partial_sigs.get(0).unwrap()"),
    };

    runtime.send_p2p(event.endpoints, LnMsg::FundingCreated(funding_created))?;
    Ok(ChannelPropose::Funding)
}

fn complete_funding(
    event: Event<BusMsg>,
    runtime: &mut Runtime,
) -> Result<ChannelPropose, state_machines::Error> {
    let funding_signed = match event.message {
        BusMsg::Ln(LnMsg::FundingSigned(funding_signed)) => funding_signed,
        wrong_msg => {
            return Err(Error::UnexpectedMessage(wrong_msg, Lifecycle::Funding, event.source))
        }
    };

    runtime.send_ctl(
        event.endpoints,
        ServiceId::Lnpd,
        CtlMsg::PublishFunding(funding_signed.signature),
    )?;
    Ok(ChannelPropose::Signed)
}

fn complete_signed(
    event: Event<BusMsg>,
    runtime: &mut Runtime,
) -> Result<ChannelPropose, state_machines::Error> {
    if !matches!(event.message, BusMsg::Ctl(CtlMsg::FundingPublished)) {
        return Err(Error::UnexpectedMessage(event.message, Lifecycle::Signed, event.source));
    }

    let channel = &runtime.channel;
    let funding_outpoint = channel.funding_outpoint();
    runtime.send_ctl(event.endpoints, ServiceId::Chain, CtlMsg::Track(funding_outpoint.txid))?;
    Ok(ChannelPropose::Funded)
}

fn complete_funded(
    _event: Event<BusMsg>,
    _runtime: &mut Runtime,
) -> Result<ChannelPropose, state_machines::Error> {
    todo!()
}

fn complete_locked(
    _event: Event<BusMsg>,
    _runtime: &mut Runtime,
) -> Result<(), state_machines::Error> {
    todo!()
}

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

use lnp::channel::bolt::Lifecycle;
use lnp::p2p::bolt::{ActiveChannelId, ChannelId, FundingSigned, Messages as LnMsg};
use lnp::Extension;
use lnp_rpc::ServiceId;
use microservices::cli::LogStyle;
use microservices::esb::Handler;

use super::Error;
use crate::automata::{Event, StateMachine};
use crate::bus::{AcceptChannelFrom, BusMsg, CtlMsg};
use crate::channeld::runtime::Runtime;
use crate::{Endpoints, Responder};

/// Channel proposal workflow
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug, Display)]
#[derive(StrictEncode, StrictDecode)]
pub enum ChannelAccept {
    /// remote peer proposed a new channel to accept
    #[display("ACCEPTED")]
    Accepted,

    /// signed commitment and sent it to the remote peer
    #[display("SIGNED")]
    Signed,

    /// awaiting funding transaction to be mined
    #[display("FUNDED")]
    Funded,

    /// funding transaction is mined, awaiting for the other peer confirmation of this fact
    #[display("LOCKED")]
    Locked,
}

impl StateMachine<BusMsg, Runtime> for ChannelAccept {
    type Error = Error;

    fn next(
        self,
        event: Event<BusMsg>,
        runtime: &mut Runtime,
    ) -> Result<Option<Self>, Self::Error> {
        let channel_id = runtime.state.channel.active_channel_id();
        debug!("ChannelAccept {:#} received {} event", channel_id, event.message);
        let state = match self {
            ChannelAccept::Accepted => finish_accepted(event, runtime),
            ChannelAccept::Signed => finish_signed(event, runtime),
            ChannelAccept::Funded => finish_funded(event, runtime),
            ChannelAccept::Locked => {
                finish_locked(event, runtime)?;
                info!("ChannelAccept {} has completed its work", channel_id);
                return Ok(None);
            }
        }?;
        info!("ChannelAccept {:#} switched to {} state", channel_id, state);
        Ok(Some(state))
    }
}

impl ChannelAccept {
    /// Computes channel lifecycle stage for the current channel proposal workflow stage
    pub fn lifecycle(&self) -> Lifecycle {
        match self {
            ChannelAccept::Accepted => Lifecycle::Accepted,
            ChannelAccept::Signed => Lifecycle::Signed,
            ChannelAccept::Funded => Lifecycle::Funded,
            ChannelAccept::Locked => Lifecycle::Locked,
        }
    }
}

// State transitions:

impl ChannelAccept {
    /// Constructs channel acceptance state machine
    pub fn with(
        runtime: &mut Runtime,
        endpoints: &mut Endpoints,
        request: AcceptChannelFrom,
    ) -> Result<ChannelAccept, Error> {
        let open_channel = LnMsg::OpenChannel(request.channel_req.clone());
        runtime.state.channel.update_from_peer(&open_channel)?;

        let _ = runtime.send_ctl(
            endpoints,
            ServiceId::Signer,
            CtlMsg::DeriveKeyset(request.channel_req.temporary_channel_id.into()),
        );
        Ok(ChannelAccept::Accepted)
    }

    /// Construct information message for error and client reporting
    pub fn info_message(&self, channel_id: ActiveChannelId) -> String {
        match self {
            ChannelAccept::Accepted => format!(
                "{} channel {:#} from a remote peer",
                "Accepted".ended(),
                channel_id.actor(),
            ),
            ChannelAccept::Signed => {
                format!("{} channel {:#} from a remote peer", "Signed".ended(), channel_id.actor(),)
            }
            ChannelAccept::Funded => {
                format!("{} channel {:#} from a remote peer", "Funded".ended(), channel_id.actor(),)
            }
            ChannelAccept::Locked => {
                format!("{} channel {:#} from a remote peer", "Locked".ended(), channel_id.actor(),)
            }
        }
    }
}

fn finish_accepted(event: Event<BusMsg>, runtime: &mut Runtime) -> Result<ChannelAccept, Error> {
    let accept_event = match event.message {
        BusMsg::Ctl(CtlMsg::Keyset(_, keys)) => {
            runtime.state.channel.constructor_mut().set_local_keys(keys);

            let accept_channel = runtime.state.channel.compose_accept_channel()?;
            let accept_channel = LnMsg::AcceptChannel(accept_channel);

            trace!("Notifying remote peer about channel creation");
            runtime.send_p2p(event.endpoints, accept_channel)?;
            Ok(ChannelAccept::Signed)
        }
        wrong_msg => {
            return Err(Error::UnexpectedMessage(wrong_msg, Lifecycle::Accepted, event.source))
        }
    };
    accept_event
}

fn finish_signed(event: Event<BusMsg>, runtime: &mut Runtime) -> Result<ChannelAccept, Error> {
    let signed_event = match event.message {
        BusMsg::Bolt(LnMsg::FundingCreated(funding)) => {
            let old_id = runtime
                .state
                .channel
                .temp_channel_id()
                .expect("temporary channel id always known at this stage");

            let channel_id = ChannelId::with(funding.funding_txid, funding.funding_output_index);
            debug!("Changing channel id from {} to {}", runtime.identity(), channel_id);
            runtime
                .set_identity(event.endpoints, channel_id)
                .expect("unable to change ZMQ channel identity");
            runtime.state.channel.update_from_peer(&LnMsg::FundingCreated(funding.clone()))?;

            runtime.send_ctl(event.endpoints, ServiceId::LnpBroker, CtlMsg::ChannelUpdate {
                old_id,
                new_id: channel_id,
            })?;

            runtime.send_p2p(
                event.endpoints,
                LnMsg::FundingSigned(FundingSigned { channel_id, signature: funding.signature }),
            )?;
            Ok(ChannelAccept::Funded)
        }
        wrong_msg => {
            return Err(Error::UnexpectedMessage(wrong_msg, Lifecycle::Signed, event.source))
        }
    };

    signed_event
}

fn finish_funded(event: Event<BusMsg>, runtime: &mut Runtime) -> Result<ChannelAccept, Error> {
    let funded_event = match event.message {
        BusMsg::Bolt(LnMsg::FundingLocked(funding)) => {
            // Save next per commitment point
            runtime.state.channel.update_from_peer(&LnMsg::FundingLocked(funding.clone()))?;
            trace!("Notifying runtime about channel creation");
            let _ = runtime.send_ctl(
                event.endpoints,
                ServiceId::Router,
                CtlMsg::ChannelCreated(
                    runtime.state.channel.channel_info(runtime.state.remote_id()),
                ),
            );

            // TODO: find the alternative to this. The hello is calling to force running
            // finish_locked method
            runtime.send_ctl(event.endpoints, ServiceId::Router, CtlMsg::Hello)?;

            Ok(ChannelAccept::Locked)
        }
        wrong_msg => {
            return Err(Error::UnexpectedMessage(wrong_msg, Lifecycle::Locked, event.source))
        }
    };
    funded_event
}

fn finish_locked(event: Event<BusMsg>, runtime: &mut Runtime) -> Result<(), Error> {
    let locked_event = match event.message {
        BusMsg::Ctl(CtlMsg::Hello) => {
            debug!("Funding transaction mined, notifying remote peer");
            let funding_locked = runtime.state.channel.compose_funding_locked();
            runtime.send_p2p(event.endpoints, LnMsg::FundingLocked(funding_locked))?;

            Ok(())
        }
        wrong_msg => {
            return Err(Error::UnexpectedMessage(wrong_msg, Lifecycle::Active, event.source))
        }
    };
    locked_event
}

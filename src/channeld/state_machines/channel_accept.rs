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
use lnp::Extension;

use super::Error;
use crate::channeld::runtime::Runtime;
use crate::service::LogStyle;
use crate::state_machine::{Event, StateMachine};
use crate::{rpc, ServiceId};

/// Channel proposal workflow
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug, Display)]
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

impl StateMachine<rpc::Request, Runtime> for ChannelAccept {
    type Error = Error;

    fn next(
        self,
        event: Event<rpc::Request>,
        runtime: &mut Runtime,
    ) -> Result<Option<Self>, Self::Error> {
        let channel_id = runtime.channel.active_channel_id();
        debug!("ChannelAccept {} received {} event", channel_id, event.message);
        let state = match self {
            ChannelAccept::Accepted => finish_accepted(event, runtime),
            ChannelAccept::Signed => finish_signed(event, runtime),
            ChannelAccept::Funded => finish_funded(event, runtime),
            ChannelAccept::Locked => {
                finish_locked(event, runtime)?;
                info!("ChannelPropose {} has completed its work", channel_id);
                return Ok(None);
            }
        }?;
        info!("ChannelAccept {} switched to {} state", channel_id, state);
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
    pub fn with(event: Event<rpc::Request>, runtime: &mut Runtime) -> Result<ChannelAccept, Error> {
        let request = match event.message {
            rpc::Request::AcceptChannelFrom(ref request) => request,
            msg => {
                panic!("channel_accept workflow inconsistency: starting workflow with {}", msg)
            }
        };

        let open_channel = Messages::OpenChannel(request.channel_req.clone());
        runtime.channel.update_from_peer(&open_channel)?;

        let peerd = request.peerd.clone();
        event.complete_msg_service(
            ServiceId::Peer(peerd),
            rpc::Request::PeerMessage(open_channel),
        )?;

        Ok(ChannelAccept::Accepted)
    }

    /// Construct information message for error and client reporting
    pub fn info_message(&self, channel_id: ActiveChannelId) -> String {
        match self {
            ChannelAccept::Accepted => format!(
                "{} channel {:#} from a remote peer",
                "Accepted".ended(),
                channel_id.ender(),
            ),
            _ => todo!(),
        }
    }
}

fn finish_accepted(
    event: Event<rpc::Request>,
    runtime: &mut Runtime,
) -> Result<ChannelAccept, Error> {
    todo!()
}

fn finish_signed(
    event: Event<rpc::Request>,
    runtime: &mut Runtime,
) -> Result<ChannelAccept, Error> {
    todo!()
}

fn finish_funded(
    event: Event<rpc::Request>,
    runtime: &mut Runtime,
) -> Result<ChannelAccept, Error> {
    todo!()
}

fn finish_locked(event: Event<rpc::Request>, runtime: &mut Runtime) -> Result<(), Error> { todo!() }

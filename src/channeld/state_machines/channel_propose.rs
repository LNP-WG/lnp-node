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

use amplify::{Slice32, Wrapper};
use lnp::p2p::legacy::{ChannelId, Messages, TempChannelId};
use lnp::{channel, Extension};
use microservices::esb;

use crate::channeld::runtime::Runtime;
use crate::service::LogStyle;
use crate::state_machine::{Event, StateMachine};
use crate::{rpc, ServiceId};

/// Errors for channel proposal workflow
#[derive(Debug, Display, From, Error)]
#[display(doc_comments)]
pub enum Error {
    /// the received message {0} was not expected at the {1} stage of the channel launch workflow
    UnexpectedMessage(rpc::Request, &'static str),

    /// error sending RPC request during state transition. Details: {0}
    #[from]
    Esb(esb::Error),

    /// generic LNP channel error
    #[from]
    #[display(inner)]
    Channel(channel::Error),
}

#[derive(Debug, Display)]
pub enum ChannelPropose {
    #[display("PROPOSED")]
    Proposed(TempChannelId),

    #[display("ACCEPTED")]
    Accepted(TempChannelId),

    #[display("FUNDING")]
    Funding(ChannelId),

    #[display("SIGNED")]
    Signed(ChannelId),

    #[display("FUNDED")]
    Funded(ChannelId),

    #[display("LOCKED")]
    Locked(ChannelId),
}

impl StateMachine<rpc::Request, Runtime> for ChannelPropose {
    type Error = Error;

    fn next(
        self,
        event: Event<rpc::Request>,
        runtime: &mut Runtime,
    ) -> Result<Option<Self>, Self::Error> {
        debug!("ChannelPropose {} received {} event", self.channel_id(), event.message);
        let channel_id = self.channel_id();
        let state = match self {
            ChannelPropose::Proposed(temp_channel_id) => {
                finish_proposed(event, runtime, temp_channel_id)
            }
            ChannelPropose::Accepted(temp_channel_id) => {
                finish_accepted(event, runtime, temp_channel_id)
            }
            ChannelPropose::Funding(channel_id) => finish_funding(event, runtime, channel_id),
            ChannelPropose::Signed(channel_id) => finish_signed(event, runtime, channel_id),
            ChannelPropose::Funded(channel_id) => finish_funded(event, runtime, channel_id),
            ChannelPropose::Locked(channel_id) => {
                finish_locked(event, runtime, channel_id)?;
                info!("ChannelPropose {} has completed its work", channel_id);
                return Ok(None);
            }
        }?;
        info!("ChannelPropose {} switched to {} state", channel_id, state);
        Ok(Some(state))
    }
}

impl ChannelPropose {
    /// Computes current channel id
    pub fn channel_id(&self) -> Slice32 {
        match self {
            ChannelPropose::Proposed(temp_channel_id)
            | ChannelPropose::Accepted(temp_channel_id) => temp_channel_id.into_inner(),
            ChannelPropose::Funding(channel_id)
            | ChannelPropose::Signed(channel_id)
            | ChannelPropose::Funded(channel_id)
            | ChannelPropose::Locked(channel_id) => channel_id.into_inner(),
        }
    }
}

// State transitions:

impl ChannelPropose {
    /// Constructs channel proposal state machine
    pub fn with(
        event: Event<rpc::Request>,
        runtime: &mut Runtime,
        temp_channel_id: TempChannelId,
    ) -> Result<ChannelPropose, Error> {
        let request = match event.message {
            rpc::Request::OpenChannelWith(ref request) => request,
            msg => {
                panic!("channel_propose workflow inconsistency: starting workflow with {}", msg)
            }
        };

        info!(
            "{} remote peer to {} with temp id {:#}",
            "Proposing".promo(),
            "open a channel".promo(),
            request.channel_req.temporary_channel_id.promoter()
        );

        let open_channel = Messages::OpenChannel(request.channel_req.clone());
        runtime.channel.update_from_peer(&open_channel)?;

        let peerd = request.peerd.clone();
        event.complete_msg_service(
            ServiceId::Peer(peerd),
            rpc::Request::PeerMessage(open_channel),
        )?;

        Ok(ChannelPropose::Proposed(temp_channel_id))
    }
}

fn finish_proposed(
    event: Event<rpc::Request>,
    runtime: &mut Runtime,
    temp_channel_id: TempChannelId,
) -> Result<ChannelPropose, Error> {
    todo!()
}

fn finish_accepted(
    event: Event<rpc::Request>,
    runtime: &mut Runtime,
    temp_channel_id: TempChannelId,
) -> Result<ChannelPropose, Error> {
    todo!()
}

fn finish_funding(
    event: Event<rpc::Request>,
    runtime: &mut Runtime,
    channel_id: ChannelId,
) -> Result<ChannelPropose, Error> {
    todo!()
}

fn finish_signed(
    event: Event<rpc::Request>,
    runtime: &mut Runtime,
    channel_id: ChannelId,
) -> Result<ChannelPropose, Error> {
    todo!()
}

fn finish_funded(
    event: Event<rpc::Request>,
    runtime: &mut Runtime,
    channel_id: ChannelId,
) -> Result<ChannelPropose, Error> {
    todo!()
}

fn finish_locked(
    event: Event<rpc::Request>,
    runtime: &mut Runtime,
    channel_id: ChannelId,
) -> Result<(), Error> {
    todo!()
}

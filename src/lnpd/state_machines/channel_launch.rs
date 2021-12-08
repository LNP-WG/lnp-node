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

use amplify::Slice32;
use internet2::TypedEnum;
use lnp::p2p::legacy::{ChannelId, TempChannelId};
use microservices::esb;
use psbt::Psbt;

use crate::lnpd::runtime::Runtime;
use crate::rpc::ServiceBus;
use crate::{rpc, Senders, ServiceId};

pub trait StateMachine<Message: TypedEnum, Runtime: esb::Handler<ServiceBus>> {
    type Error: std::error::Error;

    fn next(self, event: Event<Message>, runtime: &Runtime) -> Result<Self, Self::Error>;
}

pub struct Event<'esb, Message: TypedEnum> {
    senders: &'esb mut Senders,
    service: ServiceId,
    source: ServiceId,
    pub message: Message,
}

impl<'esb, Message> Event<'esb, Message>
where
    Message: TypedEnum,
{
    pub fn with(
        senders: &'esb mut Senders,
        service: ServiceId,
        source: ServiceId,
        message: Message,
    ) -> Self {
        Event { senders, service, source, message }
    }

    pub fn complete(self, message: rpc::Request) -> Result<(), esb::Error> {
        self.senders.send_to(ServiceBus::Ctl, self.service, self.source, message)
    }
}

pub enum Error {
    UnexpectedMessage(rpc::Request),
}

pub enum ChannelLauncher {
    #[display("LAUNCHING")]
    Launching(TempChannelId, rpc::request::CreateChannel),

    #[display("NEGOTIATING")]
    Negotiating(TempChannelId),

    #[display("COMMITTING")]
    Committing(TempChannelId, Psbt),

    #[display("SIGNING")]
    Signing(ChannelId, Psbt),
}

impl StateMachine<rpc::Request, Runtime> for ChannelLauncher {
    type Error = Error;

    fn next(mut self, event: Event<rpc::Request>, runtime: &Runtime) -> Result<Self, Self::Error> {
        debug!(
            "ChannelLauncher for channel {} received {} event",
            self.channel_id(),
            event.message
        );
        let state = match self {
            ChannelLauncher::Launching(temp_channel_id, request) => {
                finish_launching(event, temp_channel_id, request)
            }
            ChannelLauncher::Negotiating(temp_channel_id) => {
                finish_negotiating(event, runtime, temp_channel_id)
            }
            ChannelLauncher::Committing(temp_channel_id, psbt) => {
                finish_committing(event, temp_channel_id, psbt)
            }
            ChannelLauncher::Signing(channel_id, psbt) => finish_signing(event, channel_id, psbt),
        }?;
        info!("ChannelLauncher for channel {} switched to {} state", self.channel_id(), state);
        Ok(state)
    }
}

impl ChannelLauncher {
    pub fn with() -> Result<(), Self::Error> {}

    pub fn channel_id(&self) -> Slice32 {
        match self {
            ChannelLauncher::Launching(temp_channel_id, _)
            | ChannelLauncher::Negotiating(temp_channel_id)
            | ChannelLauncher::Committing(temp_channel_id, _) => temp_channel_id.into_inner(),
            ChannelLauncher::Signing(channel_id, _) => channel_id.into_inner(),
        }
    }
}

fn finish_launching(
    event: Event<rpc::Request>,
    temp_channel_id: TempChannelId,
    request: rpc::request::CreateChannel,
) -> Result<ChannelLauncher, Self::Error> {
    if event.message != rpc::Request::Hello {
        return Err(Error::UnexpectedMessage(message));
    }
    debug_assert_eq!(
        event.source,
        ServiceId::Channel(temp_channel_id.into()),
        "Hello RPC CTL message originating not from a channel daemon"
    );
    event.complete(rpc::Request::OpenChannelWith(request))?;
    Ok(ChannelLauncher::Negotiating(temp_channel_id))
}

fn finish_negotiating(
    event: Event<rpc::Request>,
    runtime: &Runtime,
    temp_channel_id: TempChannelId,
) -> Result<ChannelLauncher, Self::Error> {
    if !matches!(event.message, rpc::Request::ChannelFunding(_)) {
        return Err(Error::UnexpectedMessage(message));
    }
    debug_assert_eq!(
        event.source,
        ServiceId::Channel(temp_channel_id.into()),
        "Hello RPC CTL message originating not from a channel daemon"
    );
    let psbt = runtime.funding_wallet.construct_funding_psbt()?;
    event.complete(rpc::Request::FundingCreated(psbt.clone()))?;
    Ok(ChannelLauncher::Committing(temp_channel_id, psbt))
}

fn finish_committing(
    event: Event<rpc::Request>,
    temp_channel_id: TempChannelId,
    psbt: Psbt,
) -> Result<ChannelLauncher, Self::Error> {
}

fn finish_signing(
    event: Event<rpc::Request>,
    channel_id: ChannelId,
    psbt: Psbt,
) -> Result<ChannelLauncher, Self::Error> {
}

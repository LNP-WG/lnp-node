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

//! State machines help to organize complex asynchronous worflows involving multiple daemon
//! interactions.

use internet2::TypedEnum;
use microservices::{esb, rpc_connection};

use crate::rpc::ServiceBus;
use crate::{Senders, ServiceId};

/// State machine used by runtimes for managing complex asynchronous workflows:
/// - Launching and managing channel daemon by lnpd for a locally created channel;
/// - Managing channel creation by channeld for a locally created channel;
/// - Accepting channel creation by channeld;
/// - Cooperative and non-cooperative channel closings;
/// - Channel operations.
pub trait StateMachine<Message: TypedEnum, Runtime: esb::Handler<ServiceBus>>
where
    esb::Error: From<<Runtime as esb::Handler<ServiceBus>>::Error>,
{
    /// Workflow-specific error type
    type Error: std::error::Error;

    /// Move state machine to a next step in response to the provided event.
    /// At the completion of the cycle the state machine is consumed and `Ok(None)` is returned.
    fn next(
        self,
        event: Event<Message>,
        runtime: &mut Runtime,
    ) -> Result<Option<Self>, Self::Error>
    where
        Self: Sized;
}

/// Event changing state machine state, consisting of a certain P2P or PRC `message` sent from some
/// serivce `source` to the current `service`.
pub struct Event<'esb, Message: TypedEnum> {
    /// ESB API provided by a controller
    senders: &'esb mut Senders,
    /// Local service id (event receiver)
    pub service: ServiceId,
    /// Remote service id (event originator)
    pub source: ServiceId,
    /// Message that triggered the event
    pub message: Message,
}

impl<'esb, Message> Event<'esb, Message>
where
    Message: rpc_connection::Request,
{
    /// Constructs event out of the provided data
    pub fn with(
        senders: &'esb mut Senders,
        service: ServiceId,
        source: ServiceId,
        message: Message,
    ) -> Self {
        Event { senders, service, source, message }
    }

    /// Finalizes event processing by sending reply message
    pub fn complete(self, message: Message) -> Result<(), esb::Error> {
        self.senders.send_to(ServiceBus::Ctl, self.service, self.source, message)
    }

    /// Finalizes event processing by sending reply message to a specific service (different from
    /// the event originating service).
    pub fn complete_with_service(
        self,
        service: ServiceId,
        message: Message,
    ) -> Result<(), esb::Error> {
        self.senders.send_to(ServiceBus::Ctl, self.service, service, message)
    }
}

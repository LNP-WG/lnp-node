// LNP Node: node running lightning network protocol and generalized lightning
// channels.
// Written in 2020-2024 by
//     Dr. Maxim Orlovsky <orlovsky@lnp-bp.org>
//
// To the extent possible under law, the author(s) have dedicated all
// copyright and related and neighboring rights to this software to
// the public domain worldwide. This software is distributed without
// any warranty.
//
// You should have received a copy of the MIT License along with this software.
// If not, see <https://opensource.org/licenses/MIT>.

//! State machines help to organize complex asynchronous worflows involving multiple daemon
//! interactions.

use microservices::esb;

use crate::bus::{BusMsg, ServiceBus};
use crate::rpc::ServiceId;
use crate::Endpoints;

/// State machine used by runtimes for managing complex asynchronous workflows:
/// - Launching and managing channel daemon by lnpd for a locally created channel;
/// - Managing channel creation by channeld for a locally created channel;
/// - Accepting channel creation by channeld;
/// - Cooperative and non-cooperative channel closings;
/// - Channel operations.
pub trait StateMachine<Message, Runtime: esb::Handler<ServiceBus>>
where
    esb::Error<ServiceId>: From<<Runtime as esb::Handler<ServiceBus>>::Error>,
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
pub struct Event<'esb, Message> {
    /// ESB API provided by a controller
    pub endpoints: &'esb mut Endpoints,
    /// Local service id (event receiver)
    pub service: ServiceId,
    /// Remote service id (event originator)
    pub source: ServiceId,
    /// Message that triggered the event
    pub message: Message,
}

impl<'esb, Message> Event<'esb, Message>
where
    Message: Into<BusMsg>,
{
    /// Constructs event out of the provided data
    pub fn with(
        endpoints: &'esb mut Endpoints,
        service: ServiceId,
        source: ServiceId,
        message: Message,
    ) -> Self {
        Event { endpoints, service, source, message }
    }

    /// Finalizes event processing by sending reply message via CTL message bus
    pub fn complete_ctl(self, message: Message) -> Result<(), esb::Error<ServiceId>> {
        self.endpoints.send_to(ServiceBus::Ctl, self.service, self.source, message.into())
    }

    /// Finalizes event processing by sending reply message via CTL message bus to a specific
    /// service (different from the event originating service).
    pub fn complete_ctl_service(
        self,
        service: ServiceId,
        message: Message,
    ) -> Result<(), esb::Error<ServiceId>> {
        self.endpoints.send_to(ServiceBus::Ctl, self.service, service, message.into())
    }

    /// Sends a reply message via CTL message bus
    pub fn send_ctl(&mut self, message: Message) -> Result<(), esb::Error<ServiceId>> {
        self.endpoints.send_to(
            ServiceBus::Ctl,
            self.service.clone(),
            self.source.clone(),
            message.into(),
        )
    }

    /// Sends reply message via CTL message bus to a specific service (different from the event
    /// originating service).
    pub fn send_ctl_service(
        &mut self,
        service: ServiceId,
        message: Message,
    ) -> Result<(), esb::Error<ServiceId>> {
        self.endpoints.send_to(ServiceBus::Ctl, self.service.clone(), service, message.into())
    }

    /// Finalizes event processing by sending reply message via MSG message bus
    pub fn complete_msg(self, message: Message) -> Result<(), esb::Error<ServiceId>> {
        self.endpoints.send_to(ServiceBus::Msg, self.service, self.source, message.into())
    }

    /// Finalizes event processing by sending reply message via MSG message bus to a specific
    /// service (different from the event originating service).
    pub fn complete_msg_service(
        self,
        service: ServiceId,
        message: Message,
    ) -> Result<(), esb::Error<ServiceId>> {
        self.endpoints.send_to(ServiceBus::Msg, self.service, service, message.into())
    }
}

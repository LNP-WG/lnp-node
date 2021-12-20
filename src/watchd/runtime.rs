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

use lnp::p2p::legacy::Messages as LnMsg;
use microservices::esb;

use crate::bus::{BusMsg, CtlMsg, ServiceBus};
use crate::rpc::ServiceId;
use crate::{Config, Endpoints, Error, Service};

pub fn run(config: Config) -> Result<(), Error> {
    let runtime = Runtime { identity: ServiceId::Watch };

    Service::run(config, runtime, false)
}

pub struct Runtime {
    identity: ServiceId,
}

impl esb::Handler<ServiceBus> for Runtime {
    type Request = BusMsg;
    type Error = Error;

    fn identity(&self) -> ServiceId { self.identity.clone() }

    fn handle(
        &mut self,
        endpoints: &mut Endpoints,
        bus: ServiceBus,
        source: ServiceId,
        message: BusMsg,
    ) -> Result<(), Self::Error> {
        match (bus, message, source) {
            (ServiceBus::Msg, BusMsg::Ln(msg), source) => self.handle_p2p(endpoints, source, msg),
            (ServiceBus::Ctl, BusMsg::Ctl(msg), source) => self.handle_ctl(endpoints, source, msg),
            (bus, msg, _) => Err(Error::wrong_esb_msg(bus, &msg)),
        }
    }

    fn handle_err(
        &mut self,
        _: &mut Endpoints,
        _: esb::Error<ServiceId>,
    ) -> Result<(), Self::Error> {
        // We do nothing and do not propagate error; it's already being reported
        // with `error!` macro by the controller. If we propagate error here
        // this will make whole daemon panic
        Ok(())
    }
}

impl Runtime {
    fn handle_p2p(
        &mut self,
        _endpoints: &mut Endpoints,
        _source: ServiceId,
        message: LnMsg,
    ) -> Result<(), Error> {
        match message {
            _ => {
                // TODO: Process message
            }
        }
        Ok(())
    }

    fn handle_ctl(
        &mut self,
        _: &mut Endpoints,
        _: ServiceId,
        message: CtlMsg,
    ) -> Result<(), Error> {
        match message {
            wrong_msg => {
                error!("Request {} is not supported by the CTL interface", wrong_msg);
                return Err(Error::wrong_esb_msg(ServiceBus::Ctl, &wrong_msg));
            }
        }
    }
}

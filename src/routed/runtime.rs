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

use lnp::p2p::legacy::{ChannelId, Messages as LnMsg};
use lnp::router::gossip::GossipExt;
use lnp::router::Router;
use lnp::Extension;
use lnp_rpc::{ClientId, PayInvoice, RpcMsg};
use microservices::esb;

use crate::bus::{BusMsg, CtlMsg, ServiceBus};
use crate::rpc::ServiceId;
use crate::{Config, Endpoints, Error, Service};

pub fn run(config: Config) -> Result<(), Error> {
    let runtime = Runtime { identity: ServiceId::Router, router: Router::default() };

    Service::run(config, runtime, false)
}

pub struct Runtime {
    identity: ServiceId,

    router: Router<GossipExt>,

    enquirer: Option<ClientId>,
}

impl esb::Handler<ServiceBus> for Runtime {
    type Request = BusMsg;
    type Error = Error;

    fn identity(&self) -> ServiceId {
        self.identity.clone()
    }

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
            (ServiceBus::Rpc, BusMsg::Rpc(msg), ServiceId::Client(client_id)) => {
                self.handle_rpc(endpoints, client_id, msg)
            }
            (ServiceBus::Rpc, BusMsg::Rpc(_), service) => {
                unreachable!("lnpd received RPC message not from a client but from {}", service)
            }
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
        self.router.update_from_peer(&message).map_err(Error::from)
    }

    fn handle_rpc(
        &mut self,
        endpoints: &mut Endpoints,
        client_id: ClientId,
        message: RpcMsg,
    ) -> Result<(), Error> {
        match message {
            RpcMsg::PayInvoice(PayInvoice { channel_id, invoice }) => {
                self.enquirer = Some(client_id);
                self.pay_invoice(channel_id, invoice)?
            }

            wrong_msg => {
                error!("Request is not supported by the RPC interface");
                return Err(Error::wrong_esb_msg(ServiceBus::Rpc, &wrong_msg));
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

    fn pay_invoice(&mut self, channel_id: ChannelId, invoice: Invoice) -> Result<(), Error> {
        // TODO: Add private channel information from invoice to router (use dedicated PrivateRouter)
        let route = self.router.compute_route(channel_id, invoice.into())?;

        Ok(())
    }
}

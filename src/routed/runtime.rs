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

use core::convert::TryInto;
use core::time::Duration;
use std::thread::sleep;

use lnpbp::lnp::transport::zmqsocket;
use lnpbp::lnp::TypedEnum;
use lnpbp_services::esb::{self, Handler};
use lnpbp_services::node::TryService;

use crate::rpc::{Request, ServiceBus};
use crate::{Config, DaemonId, Error};

pub fn run(config: Config) -> Result<(), Error> {
    debug!("Staring RPC service runtime");
    let runtime = Runtime {
        identity: DaemonId::Routing,
    };
    let mut esb = esb::Controller::init(
        map! {
            ServiceBus::Msg => zmqsocket::Carrier::Locator(
                config.msg_endpoint.try_into()
                    .expect("Only ZMQ RPC is currently supported")
            ),
            ServiceBus::Ctl => zmqsocket::Carrier::Locator(
                config.ctl_endpoint.try_into()
                    .expect("Only ZMQ RPC is currently supported")
            )
        },
        DaemonId::router(),
        runtime,
        zmqsocket::ApiType::EsbClient,
    )?;

    sleep(Duration::from_secs(1));
    info!("routed started");
    esb.send_to(ServiceBus::Ctl, DaemonId::Lnpd, Request::Hello)?;
    esb.run_or_panic("routed");
    unreachable!()
}

pub struct Runtime {
    identity: DaemonId,
}

impl esb::Handler<ServiceBus> for Runtime {
    type Request = Request;
    type Address = DaemonId;
    type Error = Error;

    fn identity(&self) -> DaemonId {
        self.identity.clone()
    }

    fn handle(
        &mut self,
        senders: &mut esb::Senders<ServiceBus>,
        bus: ServiceBus,
        source: DaemonId,
        request: Request,
    ) -> Result<(), Self::Error> {
        match bus {
            ServiceBus::Msg => self.handle_rpc_msg(senders, source, request),
            ServiceBus::Ctl => self.handle_rpc_ctl(senders, source, request),
            _ => {
                Err(Error::NotSupported(ServiceBus::Bridge, request.get_type()))
            }
        }
    }

    fn handle_err(&mut self, _: esb::Error) -> Result<(), esb::Error> {
        // We do nothing and do not propagate error; it's already being reported
        // with `error!` macro by the controller. If we propagate error here
        // this will make whole daemon panic
        Ok(())
    }
}

impl Runtime {
    fn handle_rpc_msg(
        &mut self,
        _senders: &mut esb::Senders<ServiceBus>,
        source: DaemonId,
        request: Request,
    ) -> Result<(), Error> {
        match request {
            Request::LnpwpMessage(_message) => {
                // TODO: Process message
            }
            _ => {
                error!(
                    "MSG RPC can be only used for forwarding LNPWP messages"
                );
                return Err(Error::NotSupported(
                    ServiceBus::Msg,
                    request.get_type(),
                ));
            }
        }
        Ok(())
    }

    fn handle_rpc_ctl(
        &mut self,
        _senders: &mut esb::Senders<ServiceBus>,
        source: DaemonId,
        request: Request,
    ) -> Result<(), Error> {
        match request {
            _ => {
                error!("Request is not supported by the CTL interface");
                return Err(Error::NotSupported(
                    ServiceBus::Ctl,
                    request.get_type(),
                ));
            }
        }
    }
}

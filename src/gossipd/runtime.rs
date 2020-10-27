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

use lnpbp::lnp::TypedEnum;
use lnpbp_services::node::TryService;
use lnpbp_services::rpc;
use lnpbp_services::server::{EndpointCarrier, RpcZmqServer};

use crate::rpc::{Endpoints, Reply, Request, Rpc};
use crate::{Config, Error};

pub fn run(config: Config) -> Result<(), Error> {
    debug!("Staring RPC service runtime");
    let runtime = Runtime {};
    let rpc = RpcZmqServer::init(
        map! {
            Endpoints::Msg => EndpointCarrier::Address(
                config.msg_endpoint.try_into()
                    .expect("Only ZMQ RPC is currently supported")
            ),
            Endpoints::Ctl => EndpointCarrier::Address(
                config.ctl_endpoint.try_into()
                    .expect("Only ZMQ RPC is currently supported")
            )
        },
        runtime,
    )?;
    info!("gossipd started");
    rpc.run_or_panic("gossipd");
    unreachable!()
}

pub struct Runtime {}

impl rpc::Handler<Endpoints> for Runtime {
    type Api = Rpc;
    type Error = Error;

    fn handle(
        &mut self,
        endpoint: Endpoints,
        request: Request,
    ) -> Result<Reply, Self::Error> {
        match endpoint {
            Endpoints::Msg => self.handle_rpc_msg(request),
            Endpoints::Ctl => self.handle_rpc_ctl(request),
            _ => {
                Err(Error::NotSupported(Endpoints::Bridge, request.get_type()))
            }
        }
    }
}

impl Runtime {
    fn handle_rpc_msg(&mut self, request: Request) -> Result<Reply, Error> {
        debug!("MSG RPC request: {}", request);
        match request {
            Request::LnpwpMessage(_message) => {
                // TODO: Process message
                Ok(Reply::Success)
            }
            _ => {
                error!(
                    "MSG RPC can be only used for forwarding LNPWP messages"
                );
                Err(Error::NotSupported(Endpoints::Msg, request.get_type()))
            }
        }
    }

    fn handle_rpc_ctl(&mut self, request: Request) -> Result<Reply, Error> {
        debug!("CTL RPC request: {}", request);
        match request {
            _ => {
                error!("Request is not supported by the CTL interface");
                Err(Error::NotSupported(Endpoints::Ctl, request.get_type()))
            }
        }
    }
}

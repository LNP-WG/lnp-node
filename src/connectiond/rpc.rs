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

use lnpbp::lnp::application::{message, Messages};
use lnpbp::lnp::{SendMessage, TypedEnum};
use lnpbp_services::rpc::{Failure, Handler};

use super::Runtime;
use crate::rpc::{Endpoints, Reply, Request, Rpc};
use crate::Error;

impl Handler<Endpoints> for Runtime {
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
            Endpoints::Bridge => self.handle_bridge(request),
        }
    }
}

impl Runtime {
    fn handle_rpc_msg(&mut self, request: Request) -> Result<Reply, Error> {
        match request {
            Request::LnpwpMessage(message) => {
                // 1. Check permissions
                // 2. Forward to the remote peer
                self.sender.send_message(message)?;
                Ok(Reply::Success)
            }
            _ => Err(Error::NotSupported(Endpoints::Msg, request.get_type())),
        }
    }

    fn handle_rpc_ctl(&mut self, request: Request) -> Result<Reply, Error> {
        match request {
            Request::InitConnection => {
                self.sender.send_message(Messages::Init(message::Init {
                    global_features: none!(),
                    local_features: none!(),
                    assets: none!(),
                    unknown_tlvs: none!(),
                }))?;
                Ok(Reply::Success)
            }
            Request::PingPeer => {
                self.sender.send_message(Messages::Ping)?;
                Ok(Reply::Success)
            }
            _ => Err(Error::NotSupported(Endpoints::Ctl, request.get_type())),
        }
    }

    fn handle_bridge(&mut self, request: Request) -> Result<Reply, Error> {
        match request {
            Request::LnpwpMessage(Messages::Ping) => {
                self.sender.send_message(Messages::Ping)?;
                Ok(Reply::Success)
            }
            Request::LnpwpMessage(message) => {
                // 1. Check permissions
                // 2. Forward to the corresponding daemon
                Ok(Reply::Success)
            }
            _ => {
                Err(Error::NotSupported(Endpoints::Bridge, request.get_type()))
            }
        }
    }
}

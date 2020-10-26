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

use lnpbp_services::client::RpcClient;

use crate::rpc::{Endpoints, Request, Rpc};
use crate::{Config, Error};

pub struct Runtime {
    client: RpcClient<Endpoints, Rpc>,
}

impl Runtime {
    pub fn with(config: Config) -> Result<Self, Error> {
        debug!("Setting up RPC client...");
        let client = RpcClient::init(map! {
            Endpoints::Ctl =>
                config.ctl_endpoint.try_into()
                    .expect("Only ZMQ RPC is currently supported")

        })?;

        Ok(Self { client })
    }

    pub fn request(&mut self, req: Request) -> Result<String, Error> {
        debug!("Executing {}", req);
        let reply = self.client.request(Endpoints::Ctl, req)?;
        trace!("Reply details: {:?}", reply);
        Ok(reply.to_string())
    }
}

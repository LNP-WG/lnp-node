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
use url::Url;

use lnpbp::lnp::transport::zmqsocket;
use lnpbp_services::esb;
use lnpbp_services::rpc::EndpointCarrier;

use crate::rpc::{Endpoints, Request};
use crate::{Config, DaemonId, Error};

pub struct Runtime {
    client: esb::Controller<Endpoints, Request, Handler>,
}

impl Runtime {
    pub fn with(config: Config) -> Result<Self, Error> {
        debug!("Setting up RPC client...");
        let mut url = Url::parse(&config.ctl_endpoint.to_string()).expect(
            "Internal error: URL representation of the CTRL endpoint fails",
        );
        url.set_fragment(Some(&format!("cli={}", std::process::id())));
        let client = esb::Controller::init(
            DaemonId::Foreign(url.to_string()),
            map! {
                Endpoints::Ctl =>
                    EndpointCarrier::Address(config.ctl_endpoint.try_into()
                        .expect("Only ZMQ RPC is currently supported"))

            },
            Handler,
            zmqsocket::ApiType::EsbClient,
        )?;

        Ok(Self { client })
    }

    pub fn request(
        &mut self,
        daemon: DaemonId,
        req: Request,
    ) -> Result<(), Error> {
        debug!("Executing {}", req);
        self.client.send_to(Endpoints::Ctl, daemon, req)?;
        Ok(())
    }
}

pub struct Handler;

impl esb::Handler<Endpoints> for Handler {
    type Request = Request;
    type Address = DaemonId;
    type Error = Error;

    fn handle(
        &mut self,
        _senders: &mut esb::Senders<Endpoints>,
        _endpoint: Endpoints,
        _addr: DaemonId,
        _request: Request,
    ) -> Result<(), Error> {
        // Cli does not receive replies for now
        Ok(())
    }
}

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

use lnpbp::lnp::presentation::Encode;
use lnpbp::lnp::zmq::ApiType;
use lnpbp::lnp::{transport, NoEncryption, Session, Unmarshall, Unmarshaller};

use super::Config;
use crate::api::{self, Reply, Request};
use crate::error::BootstrapError;

pub struct Runtime {
    pub(super) config: Config,
    pub(super) context: zmq::Context,
    pub(super) session_rpc: Session<NoEncryption, transport::zmq::Connection>,
    pub(super) unmarshaller: Unmarshaller<Reply>,
}

impl Runtime {
    pub fn init(config: Config) -> Result<Self, BootstrapError> {
        let mut context = zmq::Context::new();
        let session_rpc = Session::new_zmq_unencrypted(
            ApiType::Client,
            &mut context,
            config.endpoint.clone(),
            None,
        )?;
        Ok(Self {
            config,
            context,
            session_rpc,
            unmarshaller: Reply::create_unmarshaller(),
        })
    }

    pub fn request(&mut self, request: Request) -> Result<Reply, api::Error> {
        let data = request.encode()?;
        self.session_rpc.send_raw_message(data)?;
        let raw = self.session_rpc.recv_raw_message()?;
        let reply = self.unmarshaller.unmarshall(&raw)?;
        Ok((&*reply).clone())
    }
}

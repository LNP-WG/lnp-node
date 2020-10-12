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

use amplify::TryService;
use std::any::Any;

use lnpbp::lnp::presentation::Encode;
use lnpbp::lnp::zmq::ApiType;
use lnpbp::lnp::{transport, NoEncryption, Session, Unmarshall, Unmarshaller};

use super::Config;
use crate::api::{Reply, Request};
use crate::error::{BootstrapError, RuntimeError};

pub struct Runtime {
    /// Original configuration object
    config: Config,

    /// Stored sessions
    session_rpc: Session<NoEncryption, transport::zmq::Connection>,

    /// Unmarshaller instance used for parsing RPC request
    unmarshaller: Unmarshaller<Request>,
}

impl Runtime {
    pub async fn init(config: Config) -> Result<Self, BootstrapError> {
        debug!("Opening ZMQ socket {}", config.zmq_endpoint);
        let mut context = zmq::Context::new();
        let session_rpc = Session::new_zmq_unencrypted(
            ApiType::Server,
            &mut context,
            config.zmq_endpoint.clone(),
            None,
        )?;

        Ok(Self {
            config,
            session_rpc,
            unmarshaller: Request::create_unmarshaller(),
        })
    }
}

#[async_trait]
impl TryService for Runtime {
    type ErrorType = RuntimeError;

    async fn try_run_loop(mut self) -> Result<(), Self::ErrorType> {
        loop {
            match self.run().await {
                Ok(_) => debug!("API request processing complete"),
                Err(err) => {
                    error!("Error processing API request: {}", err);
                    Err(err)?;
                }
            }
        }
    }
}

impl Runtime {
    async fn run(&mut self) -> Result<(), RuntimeError> {
        trace!("Awaiting for ZMQ RPC requests...");
        let raw = self.session_rpc.recv_raw_message()?;
        let reply = self.rpc_process(raw).await.unwrap_or_else(|err| err);
        trace!("Preparing ZMQ RPC reply: {:?}", reply);
        let data = reply.encode()?;
        trace!(
            "Sending {} bytes back to the client over ZMQ RPC",
            data.len()
        );
        self.session_rpc.send_raw_message(data)?;
        Ok(())
    }

    async fn rpc_process(&mut self, raw: Vec<u8>) -> Result<Reply, Reply> {
        trace!("Got {} bytes over ZMQ RPC", raw.len());
        let message = (&*self.unmarshaller.unmarshall(&raw)?).clone();
        debug!("Received ZMQ RPC request: {:?}", message.type_id());
        match message {
            _ => unimplemented!(),
        }
    }
}

// Lightning network protocol (LNP) daemon suite
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


use tokio::task::JoinHandle;

use lnpbp::lnp::NodeAddr;

use crate::{TryService, BootstrapError, msgbus};
use super::*;
use crate::msgbus::Multipart;


pub struct Runtime {
    config: Config,
    context: zmq::Context,
    api_socket: zmq::Socket,
    sub_socket: zmq::Socket,
}

impl Runtime {
    pub async fn init(config: Config) -> Result<Self, BootstrapError> {
        let context = zmq::Context::new();

        debug!("Opening API socket to wired on {} ...", config.msgbus_peer_api_addr);
        let api_socket = context.socket(zmq::PUB)
            .map_err(|e| BootstrapError::PublishingError(e))?;
        api_socket.bind(config.msgbus_peer_api_addr.as_str())
            .map_err(|e| BootstrapError::PublishingError(e))?;

        debug!("Opening push notification socket to wired on {} ...", config.msgbus_peer_sub_addr);
        let sub_socket = context.socket(zmq::SUB)
            .map_err(|e| BootstrapError::SubscriptionError(e))?;
        sub_socket.connect(config.msgbus_peer_sub_addr.as_str())
            .map_err(|e| BootstrapError::SubscriptionError(e))?;
        sub_socket.set_subscribe("".as_bytes())
            .map_err(|e| BootstrapError::SubscriptionError(e))?;

        debug!("Console is launched");
        Ok(Self {
            config,
            context,
            api_socket,
            sub_socket,
        })
    }

    pub async fn command_connect(&self, node_addr: NodeAddr) -> Result<(), msgbus::Error> {
        debug!("Performing CONNECT command to {} ...", node_addr);
        let multipart: msgbus::Multipart = msgbus::Command::Connect(msgbus::Connect { node_addr }).into();
        self.api_socket.send_multipart(multipart, 0)?;
        trace!("Request sent, awaiting response ...");
        let rep: Multipart = self.api_socket.recv_multipart(0)?
            .iter()
            .map(|vec| zmq::Message::from(vec))
            .collect();
        println!("{:#?}", rep);
        Ok(())
    }
}

#[async_trait]
impl TryService for Runtime {
    type ErrorType = tokio::task::JoinError;

    async fn try_run_loop(self) -> Result<!, Self::ErrorType> {
        loop {

        }
    }
}

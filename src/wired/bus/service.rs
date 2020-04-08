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


use std::convert::TryFrom;
use futures::TryFutureExt;

use lnpbp::lnp::Peer;
use lnpbp::lightning::bitcoin::hashes::{Hash, sha256};
use lnpbp::lightning::secp256k1;

use crate::{TryService, BootstrapError};
use crate::wired::PeerConnectionList;
use super::*;
use crate::msgbus::*;


pub struct BusService {
    config: Config,
    context: zmq::Context,
    subscriber: zmq::Socket,
    peer_connections: PeerConnectionList,
}

#[async_trait]
impl TryService for BusService {
    type ErrorType = Error;

    async fn try_run_loop(mut self) -> Result<!, Error> {
        loop {
            match self.run().await {
                Ok(_) => debug!("Message bus request processing complete"),
                Err(err) => {
                    error!("Error processing incoming bus message: {}", err);
                    Err(err)?;
                },
            }
        }
    }
}

impl BusService {
    pub fn init(config: Config,
                context: zmq::Context,
                peer_connections: PeerConnectionList
    ) -> Result<Self, BootstrapError> {
        trace!("Subscribing on message bus requests on {} ...", config.socket_addr);
        let subscriber = context.socket(zmq::REP)
            .map_err(|e| BootstrapError::SubscriptionError(e))?;
        subscriber.connect(config.socket_addr.as_str())
            .map_err(|e| BootstrapError::SubscriptionError(e))?;
        //subscriber.set_subscribe("".as_bytes())
        //    .map_err(|e| BootstrapError::SubscriptionError(e))?;
        debug!("Subscribed to the message bus requests");

        Ok(Self {
            config,
            context,
            subscriber,
            peer_connections,
        })
    }

    async fn run(&mut self) -> Result<(), Error> {
        let req: Multipart = self.subscriber
            .recv_multipart(0)
            .map_err(|err| Error::MessageBusError(err))?
            .into_iter()
            .map(zmq::Message::from)
            .collect();
        trace!("New API request");

        trace!("Received API command {:x?}, processing ... ", req[0]);
        let resp = self.proc_command(req)
            .inspect_err(|err| error!("Error processing command: {}", err))
            .await
            .unwrap_or(Command::Failure);

        trace!("Received response from command processor: `{}`; replying to client", resp);
        self.subscriber.send_multipart(Multipart::from(Command::Success), 0)?;
        debug!("Sent reply {}", Command::Success);

        Ok(())
    }

    async fn proc_command(&mut self, req: Multipart) -> Result<Command, Error> {
        use Command::*;

        let command = Command::try_from(req)?;

        match command {
            Connect(connect) => self.command_connect(connect).await,
            _ => Err(Error::UnknownCommand)
        }
    }

    async fn command_connect(&mut self, connect: Connect) -> Result<Command, Error> {
        debug!("Got CONNECT {}", connect);

        let node_secret = sha256::Hash::hash("node_secret".as_bytes());
        let ephemeral_secret = sha256::Hash::hash("ephemeral_secret".as_bytes());

        let node_secret = secp256k1::SecretKey::from_slice(&node_secret[..])?;
        let ephemeral_secret = secp256k1::SecretKey::from_slice(&ephemeral_secret[..])?;

        debug!("Connecting to peer {}", connect.node_addr);
        let peer = Peer::new_outbound(
            connect.node_addr,
            &node_secret,
            &ephemeral_secret,
        ).await?;
        trace!("Connection to peer {} completed successfully", connect.node_addr);

        Ok(Command::Success)
    }
}

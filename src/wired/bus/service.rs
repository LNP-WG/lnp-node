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


use std::str;

use crate::{Service, BootstrapError};
use crate::wired::PeerConnectionList;
use super::*;

pub struct BusService {
    config: Config,
    context: zmq::Context,
    subscriber: zmq::Socket,
    peer_connections: PeerConnectionList,
}

#[async_trait]
impl Service for BusService {
    async fn run_loop(mut self) -> ! {
        loop {
            match self.run().await {
                Ok(_) => debug!("Message bus request processing complete"),
                Err(err) => {
                    error!("Error processing incoming bus message: {}", err)
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
        let subscriber = context.socket(zmq::SUB)
            .map_err(|e| BootstrapError::SubscriptionError(e))?;
        subscriber.connect(config.socket_addr.as_str())
            .map_err(|e| BootstrapError::SubscriptionError(e))?;
        subscriber.set_subscribe("".as_bytes())
            .map_err(|e| BootstrapError::SubscriptionError(e))?;
        debug!("Subscribed to the message bus requests");

        Ok(Self {
            config,
            context,
            subscriber,
            peer_connections,
        })
    }

    async fn run(&mut self) -> Result<(), Error> {
        let req = self.subscriber
            .recv_multipart(0)
            .map_err(|err| Error::MessageBusError(err))?;
        trace!("New API request");

        let (cmd, args) = req.split_first()
            .ok_or(Error::MalformedRequest)
            .and_then(|(cmd_data, args)| {
                Ok((
                    str::from_utf8(&cmd_data[..])
                        .map_err(|_| Error::MalformedCommand)?,
                    args
                ))
            })
            .map_err(|_| Error::MalformedCommand)?;

        trace!("Received AIP command {}, processing ... ", cmd);
        match cmd {
            "CONNECT" => self.cmd_send(args),
            _ => Err(Error::UnknownCommand)
        }
    }

    fn cmd_send(&self, args: &[Vec<u8>]) -> Result<(), Error> {
        Ok(())
    }
}

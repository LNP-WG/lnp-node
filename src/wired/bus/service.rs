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
use std::convert::TryFrom;

use crate::{Service, BootstrapError};
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
impl Service for BusService {
    async fn run_loop(mut self) -> ! {
        loop {
            match self.run().await {
                Ok(_) => debug!("Message bus request processing complete"),
                Err(err) => {
                    panic!("Error processing incoming bus message: {}", err)
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
        use Command::*;

        let req: Multipart = self.subscriber
            .recv_multipart(0)
            .map_err(|err| Error::MessageBusError(err))?
            .into_iter()
            .map(zmq::Message::from)
            .collect();
        trace!("New API request");

        trace!("Received API command {:x?}, processing ... ", req[0]);
        let command = Command::try_from(req)?;

        match command {
            Connect(connect) => self.command_connect(connect),
            _ => Err(Error::UnknownCommand)
        }
    }

    fn command_connect(&self, connect: Connect) -> Result<(), Error> {
        debug!("Got CONNECT {}", connect);
        self.subscriber.send_multipart(Multipart::from(Command::Success), 0)?;
        debug!("Sent reply {}", Command::Success);
        Ok(())
    }
}

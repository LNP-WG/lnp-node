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


use std::sync::Arc;
use tokio::{
    sync::Mutex,
    net::TcpStream
};

use crate::TryService;
use crate::wired::BootstrapError;
use super::*;

pub struct PeerService {
    config: Config,
    context: zmq::Context,
    publisher: Mutex<zmq::Socket>,
    stream: Arc<TcpStream>,
}

#[async_trait]
impl TryService for PeerService {
    type ErrorType = Error;

    async fn try_run_loop(mut self) -> Result<!, Error> {
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

impl PeerService {
    pub fn init(config: Config,
                context: zmq::Context,
                stream: Arc<TcpStream>) -> Result<Self, BootstrapError> {
        let publisher = context.socket(zmq::PUB)
            .map_err(|e| BootstrapError::PublishingError(e))?;
        //publisher.bind(config.msgbus_addr.as_str())
        //    .map_err(|e| BootstrapError::PublishingError(e))?;
        let publisher = Mutex::new(publisher);

        Ok(Self {
            config,
            context,
            publisher,
            stream,
        })
    }

    async fn run(&mut self) -> Result<(), Error> {
        let reply = "OK";

        trace!("Sending `{}` notification to clients", reply);
        self.publisher
            .lock().await
            .send(zmq::Message::from(reply), 0)
            .map_err(|err| Error::PubError(err))
    }
}

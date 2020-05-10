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
use tokio::{net::tcp, net::TcpStream, sync::Mutex};

use super::*;
use crate::BootstrapError;
use crate::TryService;

pub struct PeerService {
    config: Config,
    context: zmq::Context,
    publisher: zmq::Socket,
    reader: tcp::OwnedReadHalf,
    api_service: ApiService,
}

#[async_trait]
impl TryService for PeerService {
    type ErrorType = Error;

    async fn try_run_loop(mut self) -> Result<!, Error> {
        let mut api_service = self.api_service;
        let thread = tokio::spawn(async move {
            trace!("Running peer API service for");
            api_service
                .run_or_panic(&format!("Peer API service for"))
                .await
        });

        loop {}
    }
}

impl PeerService {
    pub fn init(
        config: Config,
        context: zmq::Context,
        stream: TcpStream,
    ) -> Result<Self, BootstrapError> {
        let publisher = context
            .socket(zmq::PUB)
            .map_err(|e| BootstrapError::PublishingError(e))?;
        publisher
            .bind(&config.msgbus_push_addr)
            .map_err(|e| BootstrapError::PublishingError(e))?;

        let (reader, writer) = stream.into_split();

        let api_service = ApiService::init(&config.msgbus_api_addr, context.clone(), writer)?;

        Ok(Self {
            config,
            context,
            publisher,
            reader,
            api_service,
        })
    }

    async fn run(&mut self) -> Result<(), Error> {
        Ok(())
        /*
        let reply = "OK";

        trace!("Sending `{}` notification to clients", reply);
        self.publisher
            .lock().await
            .send(zmq::Message::from(reply), 0)
            .map_err(|err| Error::PubError(err))
         */
    }
}

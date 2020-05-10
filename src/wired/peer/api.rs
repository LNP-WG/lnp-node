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

use lnpbp::api::Multipart;
use tokio::{net::tcp, net::TcpStream};

use super::*;
use crate::msgbus::Command;
use crate::BootstrapError;
use crate::TryService;

pub struct ApiService {
    listener: zmq::Socket,
    writer: tcp::OwnedWriteHalf,
}

#[async_trait]
impl TryService for ApiService {
    type ErrorType = Error;

    async fn try_run_loop(mut self) -> Result<!, Error> {
        loop {
            match self.run().await {
                Ok(_) => debug!("Message bus request processing complete"),
                Err(err) => error!("Error processing incoming bus message: {}", err),
            }
        }
    }
}

impl ApiService {
    pub fn init(
        api_socket: &str,
        context: zmq::Context,
        writer: tcp::OwnedWriteHalf,
    ) -> Result<Self, BootstrapError> {
        let listener = context
            .socket(zmq::REP)
            .map_err(|e| BootstrapError::SubscriptionError(e))?;
        listener
            .bind(api_socket)
            .map_err(|e| BootstrapError::SubscriptionError(e))?;

        Ok(Self { listener, writer })
    }

    async fn run(&mut self) -> Result<(), Error> {
        let req: Multipart = self
            .listener
            .recv_multipart(0)
            .map_err(|err| Error::PubError(err))?
            .into_iter()
            .map(zmq::Message::from)
            .collect();

        /*
        trace!("Received peer API command {:x?}, processing ... ", req[0]);
        let resp = self
            .proc_command(req)
            .inspect_err(|err| error!("Error processing command: {}", err))
            .await
            .unwrap_or(Command::Failure);


        trace!(
            "Received response from peer command processor: `{}`; replying to client",
            resp
        );
        self.subscriber
            .send_multipart(Multipart::from(Command::Success), 0)?;
        debug!("Sent reply {}", Command::Success);
         */

        Ok(())
    }
}

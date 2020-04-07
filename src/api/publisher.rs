// Lightning network protocol (LNP) daemon
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
use tokio::sync::Mutex;

use crate::TryService;
use super::Config;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Display, Default)]
#[display_from(Debug)]
pub(super) struct Stats {
    // No stats collected yet
}

#[derive(Debug, Display)]
#[display_from(Debug)]
pub enum Error {
    APISocketError(zmq::Error),
    ParserIPCError(zmq::Error),
    UnknownData(Vec<u8>),
    UnknownResponse,
}

impl std::error::Error for Error {}

pub(super) struct PublisherService {
    config: Config,
    stats: Stats,
    publisher: zmq::Socket,
    subscriber: zmq::Socket,
    busy_flag: Arc<Mutex<bool>>,
}

#[async_trait]
impl TryService for PublisherService {
    type ErrorType = Error;

    async fn try_run_loop(mut self) -> Result<!, Error> {
        loop {
            match self.run().await {
                Ok(_) => trace!("Notification loop completed"),
                Err(err) => {
                    self.publisher
                        .send(zmq::Message::from("ERR"), 0)
                        .map_err(|e| Error::APISocketError(e))?;
                    error!("Error during notification loop {}", err)
                },
            }
        }
    }
}

impl PublisherService {
    pub(super) fn init(config: Config,
                publisher: zmq::Socket,
                subscriber: zmq::Socket,
                flag: &Arc<Mutex<bool>>) -> Self {
        Self {
            config,
            stats: Stats::default(),
            publisher,
            subscriber,
            busy_flag: flag.clone()
        }
    }

    async fn run(&mut self) -> Result<(), Error> {
        let resp = self.subscriber
            .recv_string(0)
            .map_err(|err| Error::ParserIPCError(err))?
            .map_err(|data| Error::UnknownData(data))?;
        trace!("Parser has completed with response {}; removing BUSY status", resp);

        *self.busy_flag.lock().await = false;
        let reply = match resp.as_str() {
            "RDY" => "RDY",
            "ERR" => "ERR",
            _ => Err(Error::UnknownResponse)?,
        };

        trace!("Sending `{}` notification to clients", reply);
        self.publisher
            .send(zmq::Message::from(reply), 0)
            .map_err(|err| Error::APISocketError(err))
    }
}
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
    UknownRequest,
    MalformedRequest,
    WrongNumberOfArgs,
}

impl std::error::Error for Error {}

impl Into<!> for Error {
    fn into(self) -> ! {
        panic!("Compile-time error (4)");
    }
}

pub(super) struct ResponderService {
    config: Config,
    stats: Stats,
    responder: zmq::Socket,
    parser: zmq::Socket,
    busy_flag: Arc<Mutex<bool>>,
}

#[async_trait]
impl TryService for ResponderService {
    type ErrorType = Error;

    async fn try_run_loop(mut self) -> Result<!, Error> {
        loop {
            match self.run().await {
                Ok(resp) => debug!("Client request processing completed"),
                Err(err) => {
                    self.responder
                        .send(zmq::Message::from("ERR"), 0)
                        .map_err(|e| Error::APISocketError(e))?;
                    error!("Error processing client's input: {}", err)
                },
            }
        }
    }
}

impl ResponderService {
    pub(super) fn init(config: Config,
                responder: zmq::Socket,
                parser: zmq::Socket,
                flag: &Arc<Mutex<bool>>) -> Self {
        Self {
            config,
            stats: Stats::default(),
            responder,
            parser,
            busy_flag: flag.clone()
        }
    }

    async fn run(&mut self) -> Result<(), Error> {
        let multipart = self.responder
            .recv_multipart(0)
            .map_err(|e| Error::APISocketError(e))?;
        trace!("Incoming input API request");
        self.proc_cmd(multipart)
            .await
            .and_then(|response| {
                trace!("Received response from command processor: `{}`; relaying it to client",
                       response.as_str().expect("We know that message is always string"));
                self.responder
                    .send(response, 0)
                    .map_err(|err| { Error::APISocketError(err) })
            })
    }

    async fn proc_cmd(&mut self, multipart: Vec<Vec<u8>>) -> Result<zmq::Message, Error> {
        use std::str;

        let (command, multipart) = multipart.split_first()
            .ok_or(Error::WrongNumberOfArgs)?;
        let cmd = str::from_utf8(&command[..]).map_err(|_| Error::MalformedRequest)?;
        debug!("Processing {} command from client ...", cmd);
        match cmd {
            "CLEAR" => self.proc_cmd_clear(multipart).await,
            "BLOCK" => self.proc_cmd_blck(multipart, false).await,
            "BLOCKS" => self.proc_cmd_blck(multipart, true).await,
            // TODO: Add support for other commands
            _ => Err(Error::UknownRequest),
        }
    }

    async fn proc_cmd_blck(&mut self, multipart: &[Vec<u8>], multiple: bool) -> Result<zmq::Message, Error> {
        let block_data = match (multipart.first(), multipart.len()) {
            (Some(data), 1) => Ok(data),
            (_, _) => Err(Error::WrongNumberOfArgs),
        }?;

        if *self.busy_flag.lock().await {
            trace!("Parser service is still busy, returning client `BUSY` status");
            return Ok(zmq::Message::from("BUSY"));
        }

        trace!("Sending block(s) data to parser service, {} bytes", block_data.len());
        *self.busy_flag.lock().await = true;
        self.parser
            .send_multipart(vec![
            zmq::Message::from(if multiple { "BLOCKS" } else { "BLOCK" }),
            zmq::Message::from(block_data)
        ], 0)
            .map_err(|err| Error::ParserIPCError(err))?;

        trace!("Reading response from parser service");
        let msg = self.parser
            .recv_msg(0)
            .map_err(|err| Error::ParserIPCError(err))?;
        if Some("ERR") == msg.as_str() {
            *self.busy_flag.lock().await = false;
        }
        Ok(msg)
    }

    async fn proc_cmd_clear(&mut self, multipart: &[Vec<u8>]) -> Result<zmq::Message, Error> {
        if !multipart.is_empty() {
            Err(Error::WrongNumberOfArgs)?
        }

        self.parser
            .send_multipart(vec![zmq::Message::from("CLEAR")], 0)
            .map_err(|err| Error::ParserIPCError(err))?;

        trace!("Reading response from parser service");
        let msg = self.parser
            .recv_msg(0)
            .map_err(|err| Error::ParserIPCError(err))?;

        Ok(msg)
    }
}
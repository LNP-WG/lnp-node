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

use core::convert::TryInto;
use std::thread::sleep;
use std::time::Duration;
use url::Url;

use lnpbp::lnp::ZmqType;
use lnpbp_services::esb;

use crate::rpc::request::OptionDetails;
use crate::rpc::{Request, ServiceBus};
use crate::{Config, Error, LogStyle, ServiceId};

pub struct Runtime {
    esb: esb::Controller<ServiceBus, Request, Handler>,
}

impl Runtime {
    pub fn with(config: Config) -> Result<Self, Error> {
        debug!("Setting up RPC client...");
        let mut url = Url::from(&config.ctl_endpoint);
        url.set_fragment(Some(&format!("cli={}", std::process::id())));
        let identity = ServiceId::client(url);
        let bus_config = esb::BusConfig::with_locator(
            config
                .ctl_endpoint
                .try_into()
                .expect("Only ZMQ RPC is currently supported"),
            Some(ServiceId::router()),
        );
        let esb = esb::Controller::with(
            map! {
                ServiceBus::Ctl => bus_config
            },
            Handler { identity },
            ZmqType::RouterConnect,
        )?;

        // We have to sleep in order for ZMQ to bootstrap
        sleep(Duration::from_secs_f32(0.1));

        Ok(Self { esb })
    }

    pub fn request(
        &mut self,
        daemon: ServiceId,
        req: Request,
    ) -> Result<(), Error> {
        debug!("Executing {}", req);
        self.esb.send_to(ServiceBus::Ctl, daemon, req)?;
        Ok(())
    }

    pub fn report_progress(&mut self) -> Result<usize, Error> {
        let mut counter = 0;
        let mut finished = false;
        while !finished {
            finished = true;
            for (_, _, rep) in self.esb.recv_poll()? {
                counter += 1;
                match rep {
                    Request::Failure(fail) => {
                        error!(
                            "{}: {}",
                            "Request failure".err(),
                            fail.err_details()
                        );
                        Err(Error::from(fail))?
                    }
                    Request::Progress(info) => {
                        info!("{}", info.progress());
                        finished = false;
                    }
                    Request::Success(OptionDetails(Some(info))) => {
                        info!("{}{}", "Success: ".ended(), info.ender());
                    }
                    Request::Success(OptionDetails(None)) => {
                        info!("{}", "Success".ended());
                    }
                    other => {
                        error!(
                            "{}: {}",
                            "Unexpected report".err(),
                            other.err_details()
                        );
                        Err(Error::Other(s!("Unexpected server response")))?
                    }
                }
            }
        }
        Ok(counter)
    }
}

pub struct Handler {
    identity: ServiceId,
}

impl esb::Handler<ServiceBus> for Handler {
    type Request = Request;
    type Address = ServiceId;
    type Error = Error;

    fn identity(&self) -> ServiceId {
        self.identity.clone()
    }

    fn handle(
        &mut self,
        _senders: &mut esb::SenderList<ServiceBus, ServiceId>,
        _bus: ServiceBus,
        _addr: ServiceId,
        _request: Request,
    ) -> Result<(), Error> {
        // Cli does not receive replies for now
        Ok(())
    }

    fn handle_err(&mut self, err: esb::Error) -> Result<(), esb::Error> {
        // We simply propagate the error since it's already being reported
        Err(err)?
    }
}

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

use amplify::Wrapper;
use core::convert::TryInto;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::io;
use std::process;

use lnpbp::bitcoin::hashes::hex::ToHex;
use lnpbp::lnp::transport::zmqsocket;
use lnpbp::lnp::{message, ChannelId, Messages, TypedEnum};
use lnpbp_services::esb::{self, Handler};
use lnpbp_services::node::TryService;

use crate::rpc::{request, Request, ServiceBus};
use crate::{Config, DaemonId, Error};

pub fn run(config: Config) -> Result<(), Error> {
    debug!("Staring RPC service runtime");
    let runtime = Runtime {
        identity: DaemonId::Lnpd,
        opening_channels: none!(),
    };
    let esb = esb::Controller::init(
        map! {
            ServiceBus::Msg => zmqsocket::Carrier::Locator(
                config.msg_endpoint.try_into()
                    .expect("Only ZMQ RPC is currently supported")
            ),
            ServiceBus::Ctl => zmqsocket::Carrier::Locator(
                config.ctl_endpoint.try_into()
                    .expect("Only ZMQ RPC is currently supported")
            )
        },
        DaemonId::router(),
        runtime,
        zmqsocket::ApiType::EsbService,
    )?;
    info!("lnpd started");
    esb.run_or_panic("lnpd");
    unreachable!()
}

pub struct Runtime {
    identity: DaemonId,
    opening_channels: HashMap<DaemonId, (DaemonId, message::OpenChannel)>,
}

impl esb::Handler<ServiceBus> for Runtime {
    type Request = Request;
    type Address = DaemonId;
    type Error = Error;

    fn identity(&self) -> DaemonId {
        self.identity.clone()
    }

    fn handle(
        &mut self,
        senders: &mut esb::Senders<ServiceBus>,
        bus: ServiceBus,
        source: DaemonId,
        request: Request,
    ) -> Result<(), Self::Error> {
        match bus {
            ServiceBus::Msg => self.handle_rpc_msg(senders, source, request),
            ServiceBus::Ctl => self.handle_rpc_ctl(senders, source, request),
            _ => {
                Err(Error::NotSupported(ServiceBus::Bridge, request.get_type()))
            }
        }
    }

    fn handle_err(&mut self, _: esb::Error) -> Result<(), esb::Error> {
        // We do nothing and do not propagate error; it's already being reported
        // with `error!` macro by the controller. If we propagate error here
        // this will make whole daemon panic
        Ok(())
    }
}

impl Runtime {
    fn handle_rpc_msg(
        &mut self,
        senders: &mut esb::Senders<ServiceBus>,
        source: DaemonId,
        request: Request,
    ) -> Result<(), Error> {
        match request {
            Request::LnpwpMessage(Messages::OpenChannel(open_channel)) => {
                info!("Creating channel by peer request from {}", source);
                self.open_channel(senders, source, open_channel)?;
            }

            Request::LnpwpMessage(_) => {
                // Ignore the rest of LN peer messages
            }

            _ => {
                error!(
                    "MSG RPC can be only used for forwarding LNPWP messages"
                );
                return Err(Error::NotSupported(
                    ServiceBus::Msg,
                    request.get_type(),
                ));
            }
        }
        Ok(())
    }

    fn handle_rpc_ctl(
        &mut self,
        senders: &mut esb::Senders<ServiceBus>,
        source: DaemonId,
        request: Request,
    ) -> Result<(), Error> {
        match request {
            Request::CreateChannel(request::CreateChannel {
                channel_req,
                connectiond,
            }) => {
                info!("Creating channel by request from {}", source);
                self.open_channel(senders, connectiond, channel_req)?;
            }

            Request::Hello => {
                // Ignoring; this is used to set remote identity at ZMQ level
                trace!("{} says hello", source);

                if let Some((connectiond, open_channel)) =
                    self.opening_channels.get(&source)
                {
                    // Tell channeld channel options and link it with the
                    // connection daemon
                    debug!(
                        "Daemon {} is known: we spawned it to create a channel. \
                         Ordering channel creation", source
                    );
                    senders.send_to(
                        ServiceBus::Ctl,
                        self.identity(),
                        source.clone(),
                        Request::CreateChannel(request::CreateChannel {
                            channel_req: open_channel.clone(),
                            connectiond: connectiond.clone(),
                        }),
                    )?;
                }
            }

            _ => {
                error!("Request is not supported by the CTL interface");
                return Err(Error::NotSupported(
                    ServiceBus::Ctl,
                    request.get_type(),
                ));
            }
        }

        Ok(())
    }

    fn open_channel(
        &mut self,
        _senders: &mut esb::Senders<ServiceBus>,
        source: DaemonId,
        open_channel: message::OpenChannel,
    ) -> Result<(), Error> {
        debug!("Instantiating channeld...");

        // Start channeld
        launch("channeld", &[open_channel.temporary_channel_id.to_hex()])
            .and_then(|child| {
                debug!(
                    "New instance of channeld launched with PID {}",
                    child.id()
                );
                Ok(())
            })?;

        self.opening_channels.insert(
            DaemonId::Channel(ChannelId::from_inner(
                open_channel.temporary_channel_id.into_inner(),
            )),
            (source, open_channel),
        );
        debug!("Awaiting for channeld to connect...");

        Ok(())
    }
}

fn launch(
    name: &str,
    args: impl IntoIterator<Item = impl AsRef<OsStr>>,
) -> io::Result<process::Child> {
    let mut bin_path = std::env::current_exe().map_err(|err| {
        error!("Unable to detect binary directory: {}", err);
        err
    })?;
    bin_path.pop();

    bin_path.push(name);
    #[cfg(target_os = "windows")]
    bin_path.set_extension("exe");

    debug!(
        "Launching {} as a separate process using `{}` as binary",
        name,
        bin_path.to_string_lossy()
    );

    let mut cmd = process::Command::new(bin_path);
    cmd.args(std::env::args().skip(1)).args(args);
    trace!("Executing `{:?}`", cmd);
    cmd.spawn().map_err(|err| {
        error!("Error launching {}: {}", name, err);
        err
    })
}

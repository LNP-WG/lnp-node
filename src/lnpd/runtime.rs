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
use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::io;
use std::process;

use lnpbp::bitcoin::hashes::hex::ToHex;
use lnpbp::lnp::{
    message, ChannelId, Messages, NodeAddr, RemoteSocketAddr, TypedEnum,
};
use lnpbp_services::esb::{self, Handler};

use crate::rpc::request::{
    IntoProgressOrFalure, IntoSuccessOrFalure, OptionDetails,
};
use crate::rpc::{request, Request, ServiceBus};
use crate::{Config, Error, LogStyle, Service, ServiceId};
use std::convert::TryFrom;
use std::net::SocketAddr;

pub fn run(config: Config) -> Result<(), Error> {
    let runtime = Runtime {
        identity: ServiceId::Lnpd,
        connections: none!(),
        channels: none!(),
        spawning_services: none!(),
        opening_channels: none!(),
        accepting_channels: none!(),
    };

    Service::run(config, runtime, true)
}

pub struct Runtime {
    identity: ServiceId,
    connections: HashSet<NodeAddr>,
    channels: HashSet<ChannelId>,
    spawning_services: HashSet<ServiceId>,
    opening_channels: HashMap<ServiceId, request::ChannelParams>,
    accepting_channels: HashMap<ServiceId, request::ChannelParams>,
}

impl esb::Handler<ServiceBus> for Runtime {
    type Request = Request;
    type Address = ServiceId;
    type Error = Error;

    fn identity(&self) -> ServiceId {
        self.identity.clone()
    }

    fn handle(
        &mut self,
        senders: &mut esb::SenderList<ServiceBus, ServiceId>,
        bus: ServiceBus,
        source: ServiceId,
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
        _senders: &mut esb::SenderList<ServiceBus, ServiceId>,
        source: ServiceId,
        request: Request,
    ) -> Result<(), Error> {
        match request {
            Request::Hello => {
                // Ignoring; this is used to set remote identity at ZMQ level
            }

            Request::LnpwpMessage(Messages::OpenChannel(open_channel)) => {
                info!("Creating channel by peer request from {}", source);
                self.create_channel(source, open_channel, true)?;
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
        senders: &mut esb::SenderList<ServiceBus, ServiceId>,
        source: ServiceId,
        request: Request,
    ) -> Result<(), Error> {
        let mut notify_cli = None;
        match request {
            Request::Hello => {
                // Ignoring; this is used to set remote identity at ZMQ level
                info!("{} daemon is {}", source.ended(), "connected".ended());

                match &source {
                    ServiceId::Lnpd => {
                        error!(
                            "{}",
                            "Unexpected another lnpd instance connection".err()
                        );
                    }
                    ServiceId::Peer(connection_id) => {
                        if self.connections.insert(connection_id.clone()) {
                            debug!(
                                "Connection daemon {} is registered; total {} \
                                 connections are known",
                                connection_id,
                                self.connections.len()
                            );
                            notify_cli = Some(Request::Success(
                                OptionDetails::with(format!(
                                    "Peer was connected to {}",
                                    connection_id
                                )),
                            ));
                        } else {
                            warn!(
                                "Connection {} was already registered; the \
                                 service probably was relaunched",
                                connection_id
                            );
                        }
                    }
                    ServiceId::Channel(channel_id) => {
                        if self.channels.insert(channel_id.clone()) {
                            debug!(
                                "Channel daemon {} is registered; total {} \
                                 channels are known",
                                channel_id,
                                self.channels.len()
                            );
                        } else {
                            warn!(
                                "Channel {} was already registered; the \
                                 service probably was relaunched",
                                channel_id
                            );
                        }
                    }
                    _ => {
                        // Ignoring the rest of daemon/client types
                    }
                }

                if let Some(channel_params) = self.opening_channels.get(&source)
                {
                    // Tell channeld channel options and link it with the
                    // connection daemon
                    debug!(
                        "Daemon {} is known: we spawned it to create a channel. \
                         Ordering channel opening", source
                    );
                    notify_cli = Some(Request::Progress(format!(
                        "Channel daemon {} operational",
                        source
                    )));
                    senders.send_to(
                        ServiceBus::Ctl,
                        self.identity(),
                        source.clone(),
                        Request::OpenChannelWith(channel_params.clone()),
                    )?;
                    self.opening_channels.remove(&source);
                } else if let Some(channel_params) =
                    self.accepting_channels.get(&source)
                {
                    // Tell channeld channel options and link it with the
                    // connection daemon
                    debug!(
                        "Daemon {} is known: we spawned it to create a channel. \
                         Ordering channel acceptance", source
                    );
                    senders.send_to(
                        ServiceBus::Ctl,
                        self.identity(),
                        source.clone(),
                        Request::AcceptChannelFrom(channel_params.clone()),
                    )?;
                    self.accepting_channels.remove(&source);
                } else if self.spawning_services.contains(&source) {
                    debug!(
                        "Daemon {} is known: we spawned it to create a new peer \
                         connection",
                        source
                    );
                    self.spawning_services.remove(&source);
                    notify_cli = Some(Request::Success(OptionDetails::with(
                        format!("Peer connected to {}", source),
                    )));
                }
            }

            Request::Listen(addr) => {
                let addr_str = addr.addr();
                info!(
                    "{} for incoming LN peer connections on {}",
                    "Starting listener".promo(),
                    addr_str
                );
                let resp = self.listen(addr);
                match resp {
                    Ok(_) => info!("Connection daemon {} for incoming LN peer connections on {}", 
                                   "listens".ended(), addr_str),
                    Err(ref err) => error!("{}", err.err())
                }
                notify_cli = Some(resp.into_success_or_failure());
            }

            Request::ConnectPeer(addr) => {
                info!(
                    "{} to remote peer {}",
                    "Connecting".promo(),
                    addr.promoter()
                );
                let resp = self.connect_peer(addr);
                match resp {
                    Ok(_) => {}
                    Err(ref err) => error!("{}", err.err()),
                }
                notify_cli = Some(resp.into_progress_or_failure());
            }

            Request::OpenChannelWith(request::ChannelParams {
                channel_req,
                peerd,
            }) => {
                info!(
                    "{} by request from {}",
                    "Creating channel".promo(),
                    source.promoter()
                );
                let resp = self.create_channel(peerd, channel_req, false);
                match resp {
                    Ok(_) => {}
                    Err(ref err) => error!("{}", err.err()),
                }
                notify_cli = Some(resp.into_progress_or_failure());
            }

            _ => {
                error!(
                    "{}",
                    "Request is not supported by the CTL interface".err()
                );
                return Err(Error::NotSupported(
                    ServiceBus::Ctl,
                    request.get_type(),
                ));
            }
        }

        if let Some(resp) = notify_cli {
            senders.send_to(ServiceBus::Ctl, ServiceId::Lnpd, source, resp)?;
        }

        Ok(())
    }

    fn listen(&mut self, addr: RemoteSocketAddr) -> Result<String, Error> {
        if let RemoteSocketAddr::Ftcp(inet) = addr {
            let socket_addr = SocketAddr::try_from(inet)?;
            let ip = socket_addr.ip();
            let port = socket_addr.port();

            debug!("Instantiating peerd...");

            // Start channeld
            let child = launch(
                "peerd",
                &["--listen", &ip.to_string(), "--port", &port.to_string()],
            )?;
            let msg = format!(
                "New instance of peerd launched with PID {}",
                child.id()
            );
            debug!("{}", msg);
            Ok(msg)
        } else {
            Err(Error::Other(s!(
                "Only TCP is supported for now as an overlay protocol"
            )))
        }
    }

    fn connect_peer(
        &mut self,
        node_endpoint: NodeAddr,
    ) -> Result<String, Error> {
        debug!("Instantiating peerd...");

        // Start channeld
        let child =
            launch("peerd", &["--connect", &node_endpoint.to_string()])?;
        let msg =
            format!("New instance of peerd launched with PID {}", child.id());
        debug!("{}", msg);
        Ok(msg)
    }

    fn create_channel(
        &mut self,
        source: ServiceId,
        open_channel: message::OpenChannel,
        accept: bool,
    ) -> Result<String, Error> {
        debug!("Instantiating channeld...");

        // Start channeld
        let child =
            launch("channeld", &[open_channel.temporary_channel_id.to_hex()])?;
        let msg = format!(
            "New instance of channeld launched with PID {}",
            child.id()
        );
        debug!("{}", msg);

        let list = if accept {
            &mut self.accepting_channels
        } else {
            &mut self.opening_channels
        };
        list.insert(
            ServiceId::Channel(ChannelId::from_inner(
                open_channel.temporary_channel_id.into_inner(),
            )),
            request::ChannelParams {
                channel_req: open_channel,
                peerd: source,
            },
        );
        debug!("Awaiting for channeld to connect...");

        Ok(msg)
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

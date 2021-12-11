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

use std::collections::{HashMap, HashSet};
use std::convert::TryFrom;
use std::ffi::OsStr;
use std::net::SocketAddr;
use std::time::{Duration, SystemTime};
use std::{io, process};

use amplify::{DumbDefault, Wrapper};
use bitcoin::hashes::hex::ToHex;
use bitcoin::secp256k1;
use internet2::{NodeAddr, RemoteSocketAddr, TypedEnum};
use lnp::bolt::{CommonParams, Keyset, PeerParams, Policy};
use lnp::p2p::legacy::{ChannelId, Messages, TempChannelId};
use lnpbp::chain::Chain;
use microservices::esb::{self, Handler};
use wallet::address::AddressCompat;

use crate::lnpd::funding_wallet::{self, FundingWallet};
use crate::lnpd::state_machines::ChannelLauncher;
use crate::opts::LNP_NODE_FUNDING_WALLET;
use crate::rpc::request::{Failure, FundsInfo, NodeInfo, OptionDetails, ToProgressOrFalure};
use crate::rpc::{request, Request, ServiceBus};
use crate::state_machine::{Event, StateMachine};
use crate::{Config, Error, LogStyle, Senders, Service, ServiceId};

pub fn run(config: Config, node_id: secp256k1::PublicKey) -> Result<(), Error> {
    let mut wallet_path = config.data_dir.clone();
    wallet_path.push(LNP_NODE_FUNDING_WALLET);
    debug!("Loading funding wallet from '{}'", wallet_path.display());
    let funding_wallet = FundingWallet::with(&config.chain, wallet_path, &config.electrum_url)?;
    info!("Funding wallet: {}", funding_wallet.descriptor());

    let runtime = Runtime {
        identity: ServiceId::Lnpd,
        node_id,
        chain: config.chain.clone(),
        listens: none!(),
        started: SystemTime::now(),
        funding_wallet,
        // TODO: Read params from config
        channel_params: (Policy::default(), CommonParams::default(), PeerParams::default()),
        connections: none!(),
        channels: none!(),
        spawning_peers: none!(),
        creating_channels: none!(),
        accepting_channels: Default::default(),
    };

    Service::run(config, runtime, true)
}

pub struct Runtime {
    identity: ServiceId,
    node_id: secp256k1::PublicKey,
    chain: Chain,
    listens: HashSet<RemoteSocketAddr>,
    started: SystemTime,
    pub(super) funding_wallet: FundingWallet,
    pub(super) channel_params: (Policy, CommonParams, PeerParams),
    connections: HashSet<NodeAddr>,
    channels: HashSet<ChannelId>,
    spawning_peers: HashMap<ServiceId, ServiceId>,
    creating_channels: HashMap<ServiceId, ChannelLauncher>,
    accepting_channels: HashMap<ServiceId, request::AcceptChannelFrom>,
}

impl esb::Handler<ServiceBus> for Runtime {
    type Request = Request;
    type Address = ServiceId;
    type Error = Error;

    fn identity(&self) -> ServiceId { self.identity.clone() }

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
            _ => Err(Error::NotSupported(ServiceBus::Bridge, request.get_type())),
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
        _senders: &mut Senders,
        source: ServiceId,
        request: Request,
    ) -> Result<(), Error> {
        let message = match request {
            Request::Hello => {
                // Ignoring; this is used to set remote identity at ZMQ level
                return Ok(());
            }
            Request::PeerMessage(message) => message,
            _ => {
                error!("MSG RPC can be only used for forwarding LN P2P messages");
                return Err(Error::NotSupported(ServiceBus::Msg, request.get_type()));
            }
        };

        let remote_peer = match source {
            ServiceId::Peer(node_addr) => node_addr,
            service => {
                unreachable!("lnpd received peer message not from a peerd but from {}", service)
            }
        };

        match message {
            Messages::OpenChannel(open_channel) => {
                info!("Creating channel by peer request from {}", remote_peer);
                self.launch_channeld(open_channel.temporary_channel_id)?;
                let channeld_id = ServiceId::Channel(open_channel.temporary_channel_id.into());
                let accept_channel = request::AcceptChannelFrom {
                    remote_peer,
                    report_to: None,
                    channel_req: open_channel,
                    policy: self.channel_params.0.clone(),
                    common_params: self.channel_params.1,
                    local_params: self.channel_params.2,
                    local_keys: self.new_channel_keyset(),
                };
                self.accepting_channels.insert(channeld_id, accept_channel);
            }
            _ => {} // nothing to do
        }
        Ok(())
    }

    fn handle_rpc_ctl(
        &mut self,
        senders: &mut Senders,
        source: ServiceId,
        request: Request,
    ) -> Result<(), Error> {
        let mut notify_cli = None;
        match request {
            Request::Hello => {
                info!("{} daemon is {}", source.ended(), "connected".ended());

                match &source {
                    ServiceId::Lnpd => {
                        error!("{}", "Unexpected another lnpd instance connection".err());
                    }
                    ServiceId::Peer(connection_id) => {
                        if self.connections.insert(connection_id.clone()) {
                            info!(
                                "Connection {} is registered; total {} connections are known",
                                connection_id,
                                self.connections.len()
                            );
                        } else {
                            warn!(
                                "Connection {} was already registered; the service probably was \
                                 relaunched",
                                connection_id
                            );
                        }
                    }
                    ServiceId::Channel(channel_id) => {
                        if self.channels.insert(channel_id.clone()) {
                            info!(
                                "Channel {} is registered; total {} channels are known",
                                channel_id,
                                self.channels.len()
                            );
                        } else {
                            warn!(
                                "Channel {} was already registered; the service probably was \
                                 relaunched",
                                channel_id
                            );
                        }
                    }
                    _ => {
                        // Ignoring the rest of daemon/client types
                    }
                }

                if let Some(channel_launcher) = self.creating_channels.remove(&source) {
                    // Tell channeld channel options and link it with the peer daemon
                    debug!(
                        "Daemon {} is known: we spawned it to create a channel. Ordering channel \
                         opening",
                        source
                    );
                    let event =
                        Event::with(senders, self.identity(), source.clone(), Request::Hello);
                    if let Some(channel_launcher) = channel_launcher.next(event, self)? {
                        self.creating_channels.insert(source, channel_launcher);
                    }
                } else if let Some(accept_channel) = self.accepting_channels.get(&source) {
                    // Tell channeld channel options and link it with the peer daemon
                    debug!(
                        "Daemon {} is known: we spawned it to create a channel. Ordering channel \
                         acceptance",
                        source
                    );
                    senders.send_to(
                        ServiceBus::Ctl,
                        self.identity(),
                        source.clone(),
                        Request::AcceptChannelFrom(accept_channel.clone()),
                    )?;
                    self.accepting_channels.remove(&source);
                } else if let Some(enquirer) = self.spawning_peers.get(&source) {
                    debug!(
                        "Daemon {} is known: we spawned it to create a new peer connection by a \
                         request from {}",
                        source, enquirer
                    );
                    notify_cli = Some((
                        Some(enquirer.clone()),
                        Request::Success(OptionDetails::with(format!(
                            "Peer connected to {}",
                            source
                        ))),
                    ));
                    self.spawning_peers.remove(&source);
                }
            }

            Request::UpdateChannelId(new_id) => {
                debug!("Requested to update channel id {} on {}", source, new_id);
                if let ServiceId::Channel(old_id) = source {
                    if !self.channels.remove(&old_id) {
                        warn!("Channel daemon {} was unknown", source);
                    }
                    self.channels.insert(new_id);
                    debug!("Registered channel daemon id {}", new_id);
                } else {
                    error!("Chanel id update may be requested only by a channeld, not {}", source);
                }
            }

            Request::GetInfo => {
                senders.send_to(
                    ServiceBus::Ctl,
                    ServiceId::Lnpd,
                    source,
                    Request::NodeInfo(NodeInfo {
                        node_id: self.node_id,
                        listens: self.listens.iter().cloned().collect(),
                        uptime: SystemTime::now()
                            .duration_since(self.started)
                            .unwrap_or(Duration::from_secs(0)),
                        since: self
                            .started
                            .duration_since(SystemTime::UNIX_EPOCH)
                            .unwrap_or(Duration::from_secs(0))
                            .as_secs(),
                        peers: self.connections.iter().cloned().collect(),
                        channels: self.channels.iter().cloned().collect(),
                    }),
                )?;
            }

            Request::ListPeers => {
                senders.send_to(
                    ServiceBus::Ctl,
                    ServiceId::Lnpd,
                    source,
                    Request::PeerList(self.connections.iter().cloned().collect()),
                )?;
            }

            Request::ListChannels => {
                senders.send_to(
                    ServiceBus::Ctl,
                    ServiceId::Lnpd,
                    source,
                    Request::ChannelList(self.channels.iter().cloned().collect()),
                )?;
            }

            Request::ListFunds => {
                let funds = self.funding_wallet.list_funds()?.into_iter().try_fold(
                    bmap! {},
                    |mut acc, f| -> Result<_, Error> {
                        *acc.entry(
                            AddressCompat::from_script(
                                f.script_pubkey.as_inner(),
                                self.funding_wallet.network(),
                            )
                            .ok_or(funding_wallet::Error::NoAddressRepresentation)?,
                        )
                        .or_insert(0) += f.amount;
                        Ok(acc)
                    },
                )?;
                senders.send_to(
                    ServiceBus::Ctl,
                    ServiceId::Lnpd,
                    source,
                    Request::FundsInfo(FundsInfo {
                        bitcoin_funds: funds,
                        asset_funds: none!(),
                        next_address: self.funding_wallet.next_funding_address()?,
                    }),
                )?;
            }

            Request::Listen(addr) => {
                let addr_str = addr.addr();
                if self.listens.contains(&addr) {
                    let msg = format!("Listener on {} already exists, ignoring request", addr);
                    warn!("{}", msg.err());
                    notify_cli = Some((
                        Some(source.clone()),
                        Request::Failure(Failure { code: 1, info: msg }),
                    ));
                } else {
                    self.listens.insert(addr);
                    info!(
                        "{} for incoming LN peer connections on {}",
                        "Starting listener".promo(),
                        addr_str
                    );
                    let resp = self.listen(addr);
                    match resp {
                        Ok(_) => info!(
                            "Connection daemon {} for incoming LN peer connections on {}",
                            "listens".ended(),
                            addr_str
                        ),
                        Err(ref err) => error!("{}", err.err()),
                    }
                    senders.send_to(
                        ServiceBus::Ctl,
                        ServiceId::Lnpd,
                        source.clone(),
                        resp.to_progress_or_failure(),
                    )?;
                    notify_cli = Some((
                        Some(source.clone()),
                        Request::Success(OptionDetails::with(format!(
                            "Node {} listens for connections on {}",
                            self.node_id, addr
                        ))),
                    ));
                }
            }

            Request::ConnectPeer(addr) => {
                info!("{} to remote peer {}", "Connecting".promo(), addr.promoter());
                let resp = self.connect_peer(source.clone(), addr);
                match resp {
                    Ok(_) => {}
                    Err(ref err) => error!("{}", err.err()),
                }
                notify_cli = Some((Some(source.clone()), resp.to_progress_or_failure()));
            }

            request @ Request::CreateChannel(_) => {
                let launcher = ChannelLauncher::with(
                    Event::with(senders, self.identity(), source, request),
                    self,
                )?;
                let channeld_id = ServiceId::Channel(launcher.channel_id().into());
                self.creating_channels.insert(channeld_id, launcher);
            }

            _ => {
                error!("{}", "Request is not supported by the CTL interface".err());
                return Err(Error::NotSupported(ServiceBus::Ctl, request.get_type()));
            }
        }

        if let Some((Some(respond_to), resp)) = notify_cli {
            senders.send_to(ServiceBus::Ctl, ServiceId::Lnpd, respond_to, resp)?;
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
            let child =
                launch("peerd", &["--listen", &ip.to_string(), "--port", &port.to_string()])?;
            let msg = format!("New instance of peerd launched with PID {}", child.id());
            info!("{}", msg);
            Ok(msg)
        } else {
            Err(Error::Other(s!("Only TCP is supported for now as an overlay protocol")))
        }
    }

    fn connect_peer(&mut self, source: ServiceId, node_addr: NodeAddr) -> Result<String, Error> {
        debug!("Instantiating peerd...");

        // Start channeld
        let child = launch("peerd", &["--connect", &node_addr.to_string()])?;
        let msg = format!("New instance of peerd launched with PID {}", child.id());
        info!("{}", msg);

        self.spawning_peers.insert(ServiceId::Peer(node_addr), source);
        debug!("Awaiting for peerd to connect...");

        Ok(msg)
    }

    pub(super) fn new_channel_keyset(&self) -> Keyset {
        // TODO: Derive proper channel keys
        Keyset::dumb_default()
    }

    pub(super) fn launch_channeld(
        &self,
        temp_channel_id: TempChannelId,
    ) -> Result<String, io::Error> {
        debug!("Instantiating channeld...");
        let child = launch("channeld", &[temp_channel_id.to_hex()])?;
        let report = format!("New instance of channeld launched with PID {}", child.id());
        info!("{}", report);
        Ok(report)
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

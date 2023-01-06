// LNP Node: node running lightning network protocol and generalized lightning
// channels.
// Written in 2020-2022 by
//     Dr. Maxim Orlovsky <orlovsky@lnp-bp.org>
//
// To the extent possible under law, the author(s) have dedicated all
// copyright and related and neighboring rights to this software to
// the public domain worldwide. This software is distributed without
// any warranty.
//
// You should have received a copy of the MIT License along with this software.
// If not, see <https://opensource.org/licenses/MIT>.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use amplify::{DumbDefault, Wrapper};
use bitcoin::Txid;
use bitcoin_scripts::address::AddressCompat;
use internet2::addr::{NodeAddr, NodeId};
use lnp::addr::LnpAddr;
use lnp::channel::bolt::{CommonParams, LocalKeyset, PeerParams, Policy};
use lnp::p2p;
use lnp::p2p::bolt::{
    ActiveChannelId, ChannelId, ChannelReestablish, Messages as LnMsg, TempChannelId,
};
use lnp::p2p::Protocol;
use lnp_rpc::{FailureCode, ListenAddr};
use microservices::cli::LogStyle;
use microservices::esb::{self, ClientId, Handler};
use microservices::peer::PeerSocket;
use microservices::util::OptionDetails;
use microservices::{DaemonHandle, LauncherError};

use crate::automata::{Event, StateMachine};
use crate::bus::{
    AcceptChannelFrom, BusMsg, CtlMsg, IntoSuccessOrFalure, ServiceBus, Status, ToProgressOrFalure,
};
use crate::lnpd::automata::ChannelLauncher;
use crate::lnpd::daemons::{read_node_key_file, Daemon};
use crate::lnpd::funding::{self, FundingWallet};
use crate::rpc::{Failure, FundsInfo, ListPeerInfo, NodeInfo, RpcMsg, ServiceId};
use crate::{Config, Endpoints, Error, Responder, Service, LNP_NODE_FUNDING_WALLET};

pub fn run<'a>(
    config: Config,
    key_file: PathBuf,
    listen: impl IntoIterator<Item = &'a ListenAddr>,
) -> Result<(), Error> {
    let node_id = read_node_key_file(&key_file).node_id();

    let listens = listen.into_iter().copied().collect();

    let runtime = Runtime {
        config: config.clone(),
        node_key_path: key_file,
        node_id,
        listens,
        started: SystemTime::now(),
        handles: vec![],
        funding_wallet: config.funding_wallet()?,
        channel_params: config.channel_params()?,
        bolt_connections: none!(),
        bifrost_connections: none!(),
        channels: none!(),
        spawning_peers: none!(),
        creating_channels: none!(),
        funding_channels: none!(),
        accepting_channels: none!(),
        reestablishing_channels: none!(),
    };

    Service::run(config, runtime, true)
}

impl Config {
    fn funding_wallet(&self) -> Result<FundingWallet, funding::Error> {
        let mut wallet_path = self.data_dir.clone();
        wallet_path.push(LNP_NODE_FUNDING_WALLET);
        debug!("Loading funding wallet from '{}'", wallet_path.display());
        let funding_wallet = FundingWallet::with(&self.chain, wallet_path, &self.electrum_url)?;
        info!("Funding wallet: {}", funding_wallet.descriptor());
        Ok(funding_wallet)
    }

    fn channel_params(&self) -> Result<(Policy, CommonParams, PeerParams), Error> {
        // TODO: Read params from config
        Ok((Policy::default(), CommonParams::default(), PeerParams::default()))
    }
}

pub struct Runtime {
    pub(super) config: Config,
    node_key_path: PathBuf,
    node_id: NodeId,
    listens: HashSet<ListenAddr>,
    started: SystemTime,
    handles: Vec<DaemonHandle<Daemon>>,
    pub(super) funding_wallet: FundingWallet,
    pub(super) channel_params: (Policy, CommonParams, PeerParams),
    bolt_connections: HashSet<NodeId>,
    bifrost_connections: HashSet<NodeId>,
    channels: HashSet<ChannelId>,
    spawning_peers: HashMap<ServiceId, ClientId>,
    creating_channels: HashMap<ServiceId, ChannelLauncher>,
    funding_channels: HashMap<Txid, ChannelLauncher>,
    accepting_channels: HashMap<ServiceId, AcceptChannelFrom>,
    reestablishing_channels: HashMap<ServiceId, (NodeId, ChannelReestablish)>,
}

impl Responder for Runtime {}

impl esb::Handler<ServiceBus> for Runtime {
    type Request = BusMsg;
    type Error = Error;

    fn identity(&self) -> ServiceId { ServiceId::LnpBroker }

    fn on_ready(&mut self, _senders: &mut Endpoints) -> Result<(), Self::Error> {
        info!("Starting signer daemon...");
        self.launch_daemon(Daemon::Signd, self.config.clone())?;
        info!("Starting routing daemon...");
        self.launch_daemon(Daemon::Routed, self.config.clone())?;
        info!("Starting chain watch daemon...");
        self.launch_daemon(Daemon::Watchd, self.config.clone())?;
        for listen_addr in self.listens.clone() {
            self.listen(
                NodeAddr::new(self.node_id, listen_addr.socket_addr),
                listen_addr.protocol,
            )?;
        }
        Ok(())
    }

    fn handle(
        &mut self,
        endpoints: &mut Endpoints,
        bus: ServiceBus,
        source: ServiceId,
        message: BusMsg,
    ) -> Result<(), Self::Error> {
        match (bus, message, source) {
            (ServiceBus::Msg, BusMsg::Bolt(msg), ServiceId::PeerBolt(remote_id)) => {
                self.handle_p2p(endpoints, remote_id, msg)
            }
            (ServiceBus::Msg, BusMsg::Bolt(_), service) => {
                unreachable!("lnpd received peer message not from a peerd but from {}", service)
            }
            (ServiceBus::Ctl, BusMsg::Ctl(msg), source) => self.handle_ctl(endpoints, source, msg),
            (ServiceBus::Rpc, BusMsg::Rpc(msg), ServiceId::Client(client_id)) => {
                self.handle_rpc(endpoints, client_id, msg)
            }
            (ServiceBus::Rpc, BusMsg::Rpc(_), service) => {
                unreachable!("lnpd received RPC message not from a client but from {}", service)
            }
            (bus, msg, _) => Err(Error::wrong_esb_msg(bus, &msg)),
        }
    }

    fn handle_err(
        &mut self,
        endpints: &mut Endpoints,
        err: esb::Error<ServiceId>,
    ) -> Result<(), Self::Error> {
        if let esb::Error::Send(source, dest, err) = err {
            // We need to report back that one of the daemons is offline so the client will not hang
            // waiting for updates forever
            error!("Daemon {} is offline", dest);
            let _ = endpints.send_to(
                ServiceBus::Ctl,
                self.identity(),
                source,
                BusMsg::Ctl(CtlMsg::EsbError { destination: dest, error: err.to_string() }),
            )?;
        }

        // We do nothing and do not propagate error; it's already being reported
        // with `error!` macro by the controller. If we propagate error here
        // this will make whole daemon panic
        Ok(())
    }
}

impl Runtime {
    fn handle_p2p(
        &mut self,
        endpoints: &mut Endpoints,
        remote_id: NodeId,
        message: LnMsg,
    ) -> Result<(), Error> {
        match message {
            // Happens when a remote peer connects to a peerd listening for incoming connections.
            // Lisnening peerd forwards this request to lnpd so it can launch a new channeld
            // instance.
            LnMsg::OpenChannel(open_channel) => {
                // TODO: Replace with state machine-based workflow
                info!("Creating channel by peer request from {}", remote_id);
                self.launch_daemon(
                    Daemon::Channeld(open_channel.temporary_channel_id.into()),
                    self.config.clone(),
                )?;
                let channeld_id = ServiceId::Channel(open_channel.temporary_channel_id.into());
                let accept_channel = AcceptChannelFrom {
                    remote_id,
                    report_to: None,
                    channel_req: open_channel,
                    policy: self.channel_params.0.clone(),
                    common_params: self.channel_params.1,
                    local_params: self.channel_params.2,
                    // TODO: Remove this field, channeld will derive keyset itself
                    local_keys: LocalKeyset::dumb_default(),
                };
                self.accepting_channels.insert(channeld_id, accept_channel);
            }

            LnMsg::ChannelReestablish(channel_reestablish) => {
                let channel_id = channel_reestablish.channel_id;
                if let Some(channeld) = self.channels.get(&channel_id) {
                    endpoints.send_to(
                        ServiceBus::Msg,
                        ServiceId::PeerBolt(remote_id),
                        ServiceId::Channel(*channeld),
                        BusMsg::Bolt(LnMsg::ChannelReestablish(channel_reestablish)),
                    )?;
                } else {
                    self.launch_daemon(
                        Daemon::Channeld(ActiveChannelId::Static(channel_id)),
                        self.config.clone(),
                    )?;
                    self.reestablishing_channels
                        .insert(ServiceId::Channel(channel_id), (remote_id, channel_reestablish));
                }
            }

            _ => {} // nothing to do for the rest of LN messages
        }
        Ok(())
    }

    fn handle_rpc(
        &mut self,
        endpoints: &mut Endpoints,
        client_id: ClientId,
        message: RpcMsg,
    ) -> Result<(), Error> {
        match message {
            RpcMsg::GetInfo => {
                let node_info = RpcMsg::NodeInfo(NodeInfo {
                    node_id: self.node_id,
                    listens: self.listens.iter().cloned().collect(),
                    uptime: SystemTime::now()
                        .duration_since(self.started)
                        .unwrap_or_else(|_| Duration::from_secs(0)),
                    since: self
                        .started
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap_or_else(|_| Duration::from_secs(0))
                        .as_secs(),
                    peers: ListPeerInfo {
                        bolt: self.bolt_connections.iter().copied().collect(),
                        bifrost: self.bifrost_connections.iter().copied().collect(),
                    },
                    channels: self.channels.iter().cloned().collect(),
                });
                self.send_rpc(endpoints, client_id, node_info)?;
            }

            RpcMsg::ListPeers => {
                let peer_list = ListPeerInfo {
                    bolt: self.bolt_connections.iter().copied().collect(),
                    bifrost: self.bifrost_connections.iter().copied().collect(),
                };
                self.send_rpc(endpoints, client_id, RpcMsg::PeerList(peer_list))?;
            }

            RpcMsg::ListChannels => {
                let channel_list = self.channels.iter().cloned().collect();
                self.send_rpc(endpoints, client_id, RpcMsg::ChannelList(channel_list))?;
            }

            RpcMsg::ListFunds => {
                let bitcoin_funds = self.available_funding()?;
                let next_address = self.funding_wallet.next_funding_address()?;
                let funds_info = FundsInfo { bitcoin_funds, asset_funds: none!(), next_address };
                self.send_rpc(endpoints, client_id, RpcMsg::FundsInfo(funds_info))?;
            }

            RpcMsg::Listen(addr) if self.listens.contains(&addr.into()) => {
                let failure = Failure {
                    code: FailureCode::Lnpd,
                    info: format!("Listener on {} already exists, ignoring request", addr),
                };
                warn!("{}", failure.info.err());
                self.send_rpc(endpoints, client_id, RpcMsg::Failure(failure))?;
            }
            RpcMsg::Listen(listen_addr) => {
                let addr_str = listen_addr.addr();
                self.listens.insert(listen_addr.into());
                info!(
                    "{} for incoming LN peer connections on {}",
                    "Starting listener".announce(),
                    addr_str
                );
                let node_addr = NodeAddr::new(self.node_id, listen_addr.socket_addr);
                let resp = self.listen(node_addr, listen_addr.protocol);
                match resp {
                    Ok(_) => info!(
                        "Connection daemon is {} for incoming LN peer connections on {}",
                        "listening".ended(),
                        addr_str
                    ),
                    Err(ref err) => error!("{}", err.err()),
                }
                self.send_rpc(endpoints, client_id, resp.into_success_or_failure())?;
            }

            RpcMsg::ConnectPeer(LnpAddr { node_addr, protocol }) => {
                // Check if the peer is already connected
                info!(
                    "{} to remote peer {} over {}",
                    "Connecting".announce(),
                    node_addr.announcer(),
                    protocol
                );
                if (protocol == p2p::Protocol::Bolt
                    && (self.spawning_peers.contains_key(&ServiceId::PeerBolt(node_addr.id))
                        || self.bolt_connections.contains(&node_addr.id)))
                    || (protocol == p2p::Protocol::Bifrost
                        && (self
                            .spawning_peers
                            .contains_key(&ServiceId::PeerBifrost(node_addr.id))
                            || self.bifrost_connections.contains(&node_addr.id)))
                {
                    info!("Already connected to a peer {}", node_addr);
                    self.send_rpc(endpoints, client_id, RpcMsg::success())?;
                    return Ok(());
                }
                // Connect otherwise
                let peer_socket = PeerSocket::Connect(node_addr.clone());
                let node_key_path = self.node_key_path.clone();
                let (peerd, peer_service_id) = match protocol {
                    p2p::Protocol::Bolt => (
                        Daemon::PeerdBolt(peer_socket, node_key_path),
                        ServiceId::PeerBolt(node_addr.id),
                    ),
                    p2p::Protocol::Bifrost => (
                        Daemon::PeerdBifrost(peer_socket, node_key_path),
                        ServiceId::PeerBifrost(node_addr.id),
                    ),
                };
                let resp = match self.launch_daemon(peerd, self.config.clone()) {
                    Ok(handle) => {
                        self.spawning_peers.insert(peer_service_id, client_id);
                        Ok(format!("Launched new instance of {}", handle))
                    }
                    Err(err) => {
                        error!("{}", err.err());
                        Err(err)
                    }
                };
                self.send_rpc(endpoints, client_id, resp.to_progress_or_failure())?;
            }

            RpcMsg::CreateChannel(create_channel) => {
                info!("Creating channel with {}", create_channel.remote_peer);
                let launcher = ChannelLauncher::with(endpoints, client_id, create_channel, self)?;
                let channeld_id = ServiceId::Channel(launcher.channel_id().into());
                self.creating_channels.insert(channeld_id, launcher);
            }

            wrong_msg => {
                error!("Request is not supported by the RPC interface");
                return Err(Error::wrong_esb_msg(ServiceBus::Rpc, &wrong_msg));
            }
        }

        Ok(())
    }

    fn handle_ctl(
        &mut self,
        endpoints: &mut Endpoints,
        source: ServiceId,
        message: CtlMsg,
    ) -> Result<(), Error> {
        match &message {
            CtlMsg::Hello => self.handle_hello(endpoints, source)?,

            CtlMsg::Keyset(service_id, _) => {
                let service_id = service_id.clone();
                let launcher = self
                    .creating_channels
                    .remove(&service_id)
                    .unwrap_or_else(|| panic!("unregistered channel launcher for {}", service_id));
                let launcher = launcher
                    .next(Event::with(endpoints, self.identity(), source, message), self)?
                    .expect("channel launcher should not be complete");
                self.creating_channels.insert(service_id, launcher);
            }

            CtlMsg::ConstructFunding(_) => {
                let launcher = self
                    .creating_channels
                    .remove(&source)
                    .unwrap_or_else(|| panic!("unregistered channel launcher for {}", source));
                let launcher = launcher
                    .next(Event::with(endpoints, self.identity(), source.clone(), message), self)?
                    .expect("channel launcher should not be complete");
                self.creating_channels
                    .insert(ChannelId::from_inner(launcher.channel_id()).into(), launcher);
            }

            CtlMsg::PublishFunding => {
                let launcher = self
                    .creating_channels
                    .remove(&source)
                    .unwrap_or_else(|| panic!("unregistered channel launcher for {}", source));
                let launcher = launcher
                    .next(Event::with(endpoints, self.identity(), source.clone(), message), self)?
                    .expect("channel launcher should not be complete");
                let txid =
                    launcher.funding_txid().expect("funding txid must be known at this stage");
                self.funding_channels.insert(txid, launcher);
            }

            CtlMsg::Signed(psbt) => {
                let txid = psbt.to_txid();
                let launcher = self
                    .funding_channels
                    .remove(&txid)
                    .unwrap_or_else(|| panic!("unregistered channel launcher for {}", source));
                let none = launcher
                    .next(Event::with(endpoints, self.identity(), source.clone(), message), self)?;
                debug_assert!(
                    matches!(none, None),
                    "Channel launcher must complete upon publishing funding transaction"
                );
            }

            CtlMsg::Error { destination, .. } | CtlMsg::EsbError { destination, .. } => {
                let launcher = self
                    .creating_channels
                    .remove(destination)
                    .unwrap_or_else(|| panic!("unregistered channel launcher for {}", destination));
                // We swallow `None` here
                let _ = launcher.next(
                    Event::with(endpoints, self.identity(), destination.clone(), message),
                    self,
                );
            }

            CtlMsg::Report(report) => {
                let msg = match &report.status {
                    Status::Progress(msg) => RpcMsg::Progress(msg.clone()),
                    Status::Success(msg) => RpcMsg::Success(msg.clone()),
                    Status::Failure(msg) => RpcMsg::Failure(msg.clone()),
                };
                // If the client is disconnected, just swallow the error - there is no reason to
                // propagate it anywhere
                if self.send_rpc(endpoints, report.client, msg).is_err() {
                    error!("Client #{} got disconnected", report.client);
                }
            }

            wrong_msg => {
                error!("Request is not supported by the CTL interface");
                return Err(Error::wrong_esb_msg(ServiceBus::Ctl, wrong_msg));
            }
        }

        Ok(())
    }

    fn handle_hello(&mut self, endpoints: &mut Endpoints, source: ServiceId) -> Result<(), Error> {
        info!("{} daemon is {}", source.ended(), "connected".ended());

        self.register_daemon(source.clone());

        if let Some(channel_launcher) = self.creating_channels.remove(&source) {
            // Tell channeld channel options and link it with the peer daemon
            debug!(
                "Ordering {} to open a channel with temp id {}",
                source,
                channel_launcher.channel_id()
            );
            let event = Event::with(endpoints, self.identity(), source.clone(), CtlMsg::Hello);
            if let Some(channel_launcher) = channel_launcher.next(event, self)? {
                self.creating_channels.insert(source, channel_launcher);
            }
        } else if let Some(accept_channel) = self.accepting_channels.remove(&source) {
            // Tell channeld channel options and link it with the peer daemon
            debug!(
                " Ordering {} to accept the channel {}",
                source, accept_channel.channel_req.temporary_channel_id
            );
            endpoints.send_to(
                ServiceBus::Ctl,
                self.identity(),
                source.clone(),
                BusMsg::Ctl(CtlMsg::AcceptChannelFrom(accept_channel)),
            )?;
        } else if let Some((remote_id, channel_reestablish)) =
            self.reestablishing_channels.remove(&source)
        {
            // Tell channeld channel options and link it with the peer daemon
            debug!(
                "Ordering {} to re-establish the channel {}",
                source, channel_reestablish.channel_id
            );
            endpoints.send_to(
                ServiceBus::Msg,
                ServiceId::PeerBolt(remote_id),
                source.clone(),
                BusMsg::Bolt(LnMsg::ChannelReestablish(channel_reestablish)),
            )?;
        } else if let Some(enquirer) = self.spawning_peers.get(&source).copied() {
            debug!("Daemon {} reported back", source);
            self.spawning_peers.remove(&source);
            let success =
                RpcMsg::Success(OptionDetails::with(format!("Peer connected to {}", source)));
            self.send_rpc(endpoints, enquirer, success)?;
        }

        Ok(())
    }

    fn register_daemon(&mut self, source: ServiceId) {
        match source {
            ServiceId::LnpBroker => {
                error!("{}", "Unexpected another lnpd instance connection".err());
            }
            ServiceId::PeerBolt(connection_id) if self.bolt_connections.insert(connection_id) => {
                info!(
                    "BOLT connection {} is registered; total {} connections are known",
                    connection_id,
                    self.bolt_connections.len()
                );
            }
            ServiceId::PeerBifrost(connection_id)
                if self.bifrost_connections.insert(connection_id) =>
            {
                info!(
                    "Bifrost connection {} is registered; total {} connections are known",
                    connection_id,
                    self.bifrost_connections.len()
                );
            }
            ServiceId::PeerBolt(connection_id) => {
                warn!(
                    "BOTL connection {} was already registered; the service probably was \
                     relaunched",
                    connection_id
                );
            }
            ServiceId::PeerBifrost(connection_id) => {
                warn!(
                    "Bifrost connection {} was already registered; the service probably was \
                     relaunched",
                    connection_id
                );
            }
            ServiceId::Channel(channel_id) if self.channels.insert(channel_id) => {
                info!(
                    "Channel {} is registered; total {} channels are known",
                    channel_id,
                    self.channels.len()
                );
            }
            ServiceId::Channel(channel_id) => {
                warn!(
                    "Channel {} was already registered; the service probably was relaunched",
                    channel_id
                );
            }
            _ => {
                // Ignoring the rest of daemon/client types
            }
        }
    }

    fn listen(
        &mut self,
        addr: NodeAddr,
        protocol: p2p::Protocol,
    ) -> Result<String, LauncherError<Daemon>> {
        info!("Starting peer connection listening daemon on {}...", addr);
        let socket = PeerSocket::Listen(addr.addr.try_into().expect("tor is not supported"));
        let node_key_path = self.node_key_path.clone();
        let daemon = match protocol {
            Protocol::Bolt => Daemon::PeerdBolt(socket, node_key_path),
            Protocol::Bifrost => Daemon::PeerdBifrost(socket, node_key_path),
        };
        let handle = self.launch_daemon(daemon, self.config.clone())?;
        Ok(format!("Launched new instance of {}", handle))
    }

    fn available_funding(&mut self) -> Result<BTreeMap<AddressCompat, u64>, Error> {
        self.funding_wallet.list_funds()?.into_iter().try_fold(
            bmap! {},
            |mut acc, f| -> Result<_, Error> {
                let addr = match AddressCompat::from_script(
                    &f.script_pubkey,
                    self.funding_wallet.network().into(),
                ) {
                    Some(address) => Ok(address),
                    _ => Err(funding::Error::NoAddressRepresentation),
                };
                *acc.entry(addr?).or_insert(0) += f.amount;
                Ok(acc)
            },
        )
    }

    pub fn update_chanel_id(&mut self, old_id: TempChannelId, new_id: ChannelId) -> bool {
        let mut known = true;
        if !self.channels.remove(&ChannelId::from(old_id)) {
            known = false;
            warn!("Temporary channel id {} was unknown", old_id);
        }
        self.channels.insert(new_id);
        info!("Channel daemon id registered to change from {} to {}", old_id, new_id);
        known
    }
}

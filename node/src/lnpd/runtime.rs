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

use std::collections::{BTreeMap, HashMap, HashSet};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use amplify::{DumbDefault, Wrapper};
use bitcoin::{secp256k1, Txid};
use internet2::addr::InetSocketAddr;
use internet2::{NodeAddr, RemoteSocketAddr};
use lnp::bolt::{CommonParams, LocalKeyset, PeerParams, Policy};
use lnp::p2p::legacy::{ChannelId, Messages as LnMessage, TempChannelId};
use microservices::esb::{self, Handler};
use wallet::address::AddressCompat;

use crate::i9n::ctl::{AcceptChannelFrom, CtlMsg, Status};
use crate::i9n::rpc::{
    Failure, FundsInfo, IntoSuccessOrFalure, NodeInfo, OptionDetails, RpcMsg, ToProgressOrFalure,
};
use crate::i9n::{BusMsg, ServiceBus};
use crate::lnpd::daemons::{Daemon, DaemonHandle};
use crate::lnpd::funding_wallet::{self, FundingWallet};
use crate::lnpd::state_machines::ChannelLauncher;
use crate::opts::LNP_NODE_FUNDING_WALLET;
use crate::peerd::supervisor::read_node_key_file;
use crate::peerd::PeerSocket;
use crate::service::ClientId;
use crate::state_machine::{Event, StateMachine};
use crate::{Config, Endpoints, Error, LogStyle, Service, ServiceId};

pub fn run(config: Config, key_file: PathBuf, listen: Option<SocketAddr>) -> Result<(), Error> {
    let mut listens = HashSet::with_capacity(1);
    if let Some(addr) = listen {
        listens.insert(RemoteSocketAddr::Ftcp(InetSocketAddr::from(addr)));
    }

    let node_id = read_node_key_file(&key_file).node_id();

    let runtime = Runtime {
        identity: ServiceId::Lnpd,
        config: config.clone(),
        node_key_path: key_file,
        node_id,
        listens,
        started: SystemTime::now(),
        handles: vec![],
        funding_wallet: config.funding_wallet()?,
        channel_params: config.channel_params()?,
        connections: none!(),
        channels: none!(),
        spawning_peers: none!(),
        creating_channels: none!(),
        funding_channels: none!(),
        accepting_channels: Default::default(),
    };

    Service::run(config, runtime, true)
}

impl Config {
    fn funding_wallet(&self) -> Result<FundingWallet, funding_wallet::Error> {
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
    identity: ServiceId,
    pub(super) config: Config,
    node_key_path: PathBuf,
    node_id: secp256k1::PublicKey,
    listens: HashSet<RemoteSocketAddr>,
    started: SystemTime,
    handles: Vec<DaemonHandle<Daemon>>,
    pub(super) funding_wallet: FundingWallet,
    pub(super) channel_params: (Policy, CommonParams, PeerParams),
    connections: HashSet<NodeAddr>,
    channels: HashSet<ChannelId>,
    spawning_peers: HashMap<ServiceId, ClientId>,
    creating_channels: HashMap<ServiceId, ChannelLauncher>,
    funding_channels: HashMap<Txid, ChannelLauncher>,
    accepting_channels: HashMap<ServiceId, AcceptChannelFrom>,
}

impl esb::Handler<ServiceBus> for Runtime {
    type Request = BusMsg;
    type Error = Error;

    fn identity(&self) -> ServiceId { self.identity.clone() }

    fn on_ready(&mut self, _senders: &mut Endpoints) -> Result<(), Self::Error> {
        info!("Starting signer daemon...");
        self.launch_daemon(Daemon::Signd, self.config.clone())?;
        for addr in self.listens.clone() {
            self.listen(addr)?;
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
            (ServiceBus::Msg, BusMsg::Ln(msg), ServiceId::Peer(remote_peer)) => {
                self.handle_p2p(endpoints, remote_peer, msg)
            }
            (ServiceBus::Msg, BusMsg::Ln(_), service) => {
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
            let dest = ServiceId::from(dest);
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
        _: &mut Endpoints,
        remote_peer: NodeAddr,
        message: LnMessage,
    ) -> Result<(), Error> {
        match message {
            // Happens when a remote peer connects to a peerd listening for incoming connections.
            // Lisnening peerd forwards this request to lnpd so it can launch a new channeld
            // instance.
            LnMessage::OpenChannel(open_channel) => {
                // TODO: Replace with state machine-based workflow
                info!("Creating channel by peer request from {}", remote_peer);
                self.launch_daemon(
                    Daemon::Channeld(open_channel.temporary_channel_id.into()),
                    self.config.clone(),
                )?;
                let channeld_id = ServiceId::Channel(open_channel.temporary_channel_id.into());
                let accept_channel = AcceptChannelFrom {
                    remote_peer,
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
                        .unwrap_or(Duration::from_secs(0)),
                    since: self
                        .started
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap_or(Duration::from_secs(0))
                        .as_secs(),
                    peers: self.connections.iter().cloned().collect(),
                    channels: self.channels.iter().cloned().collect(),
                });
                self.send_rpc(endpoints, client_id, node_info)?;
            }

            RpcMsg::ListPeers => {
                let peer_list = self.connections.iter().cloned().collect();
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

            RpcMsg::Listen(addr) if self.listens.contains(&addr) => {
                let failure = Failure {
                    code: 1, /* TODO: Update code */
                    info: format!("Listener on {} already exists, ignoring request", addr),
                };
                warn!("{}", failure.info.err());
                self.send_rpc(endpoints, client_id, RpcMsg::Failure(failure))?;
            }
            RpcMsg::Listen(addr) => {
                let addr_str = addr.addr();
                self.listens.insert(addr);
                info!(
                    "{} for incoming LN peer connections on {}",
                    "Starting listener".promo(),
                    addr_str
                );
                let resp = self.listen(addr);
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

            RpcMsg::ConnectPeer(addr) => {
                info!("{} to remote peer {}", "Connecting".promo(), addr.promoter());
                let peerd =
                    Daemon::Peerd(PeerSocket::Connect(addr.clone()), self.node_key_path.clone());
                let resp = match self.launch_daemon(peerd, self.config.clone()) {
                    Ok(handle) => {
                        self.spawning_peers.insert(ServiceId::Peer(addr.into()), client_id);
                        Ok(format!("Launched new instance of {}", handle))
                    }
                    Err(err) => {
                        error!("{}", err.err());
                        Err(Error::from(err))
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

    #[inline]
    pub(crate) fn send_rpc(
        &self,
        endpoints: &mut Endpoints,
        client_id: ClientId,
        message: impl Into<RpcMsg>,
    ) -> Result<(), esb::Error<ServiceId>> {
        endpoints.send_to(
            ServiceBus::Rpc,
            ServiceId::Lnpd,
            ServiceId::Client(client_id),
            BusMsg::Rpc(message.into()),
        )
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
                    .expect(&format!("unregistered channel launcher for {}", service_id));
                let launcher = launcher
                    .next(Event::with(endpoints, self.identity(), source, message), self)?
                    .expect("channel launcher should not be complete");
                self.creating_channels.insert(service_id, launcher);
            }

            CtlMsg::ConstructFunding(_) => {
                let launcher = self
                    .creating_channels
                    .remove(&source)
                    .expect(&format!("unregistered channel launcher for {}", source));
                let launcher = launcher
                    .next(Event::with(endpoints, self.identity(), source.clone(), message), self)?
                    .expect("channel launcher should not be complete");
                self.creating_channels.insert(source, launcher);
            }

            CtlMsg::PublishFunding => {
                let launcher = self
                    .creating_channels
                    .remove(&source)
                    .expect(&format!("unregistered channel launcher for {}", source));
                let launcher = launcher
                    .next(Event::with(endpoints, self.identity(), source.clone(), message), self)?
                    .expect("channel launcher should not be complete");
                let txid =
                    launcher.funding_txid().expect("funding txid must be known at this stage");
                self.funding_channels.insert(txid, launcher);
            }

            CtlMsg::Signed(psbt) => {
                let txid = psbt.global.unsigned_tx.txid();
                let launcher = self
                    .funding_channels
                    .remove(&txid)
                    .expect(&format!("unregistered channel launcher for {}", source));
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
                    .remove(&destination)
                    .expect(&format!("unregistered channel launcher for {}", destination));
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
                if let Err(_) = self.send_rpc(endpoints, report.client, msg) {
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
            debug!("Daemon {} reported back. Ordering channel opening", source);
            let event = Event::with(endpoints, self.identity(), source.clone(), CtlMsg::Hello);
            if let Some(channel_launcher) = channel_launcher.next(event, self)? {
                self.creating_channels.insert(source, channel_launcher);
            }
        } else if let Some(accept_channel) = self.accepting_channels.get(&source) {
            // Tell channeld channel options and link it with the peer daemon
            debug!("Daemon {} reported back. Ordering channel acceptance", source);
            endpoints.send_to(
                ServiceBus::Ctl,
                self.identity(),
                source.clone(),
                BusMsg::Ctl(CtlMsg::AcceptChannelFrom(accept_channel.clone())),
            )?;
            self.accepting_channels.remove(&source);
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
            ServiceId::Lnpd => {
                error!("{}", "Unexpected another lnpd instance connection".err());
            }
            ServiceId::Peer(connection_id) if self.connections.insert(connection_id.clone()) => {
                info!(
                    "Connection {} is registered; total {} connections are known",
                    connection_id,
                    self.connections.len()
                );
            }
            ServiceId::Peer(connection_id) => {
                warn!(
                    "Connection {} was already registered; the service probably was relaunched",
                    connection_id
                );
            }
            ServiceId::Channel(channel_id) if self.channels.insert(channel_id.clone()) => {
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

    fn listen(&mut self, addr: RemoteSocketAddr) -> Result<String, Error> {
        info!("Starting peer connection listening daemon on {}...", addr);
        let handle = self.launch_daemon(
            Daemon::Peerd(PeerSocket::Listen(addr.clone()), self.node_key_path.clone()),
            self.config.clone(),
        )?;
        Ok(format!("Launched new instance of {}", handle))
    }

    fn available_funding(&mut self) -> Result<BTreeMap<AddressCompat, u64>, Error> {
        self.funding_wallet.list_funds()?.into_iter().try_fold(
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
        )
    }

    pub fn update_chanel_id(&mut self, _old_id: TempChannelId, _new_id: ChannelId) -> bool {
        /* remove this once we add routing at microservices level
        let mut known = true;
        if !self.channels.remove(&ChannelId::from(old_id)) {
            known = false;
            warn!("Temporary channel id {} was unknown", old_id);
        }
        self.channels.insert(new_id);
        info!("Channel daemon id registered to change from {} to {}", old_id, new_id);
        known
         */
        false
    }
}

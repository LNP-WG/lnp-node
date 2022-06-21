// LNP Node: node running lightning network protocol and generalized lightning
// channels.
// Written in 2020-2022 by
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

use std::convert::TryInto;
use std::fs;
use std::io::Read;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::process::Command;

use amplify::hex::ToHex;
use bitcoin::secp256k1::{SecretKey, SECP256K1};
use internet2::addr::LocalNode;
use internet2::session::noise::FramingProtocol;
use internet2::transport;
use lnp::p2p::bolt::ActiveChannelId;
use microservices::peer::{supervisor, PeerSocket};
use microservices::{DaemonHandle, Launcher, LauncherError};

use crate::lnpd::runtime::Runtime;
use crate::service::LogStyle;
use crate::{channeld, peerd, routed, signd, watchd, Config, Error, P2pProtocol};

pub fn read_node_key_file(key_file: &Path) -> LocalNode {
    let mut file = fs::File::open(key_file).unwrap_or_else(|_| {
        panic!(
            "Unable to open key file '{}';\nplease check that the file exists and the daemon has \
             access rights to it",
            key_file.display()
        )
    });
    let mut node_secret = [0u8; 32];
    file.read_exact(&mut node_secret).expect("incorrect format of the node key file");
    let node_secret =
        SecretKey::from_slice(&node_secret).expect("incorrect format of the node key file");
    let local_node = LocalNode::with(&SECP256K1, node_secret);

    let local_id = local_node.node_id();
    info!("{}: {}", "Local node id".ended(), local_id.addr());

    local_node
}

/// Daemons that can be launched by lnpd
#[derive(Clone, Eq, PartialEq, Debug, Display)]
pub enum Daemon {
    #[display("signd")]
    Signd,

    #[display("peerd --bolt")]
    PeerdBolt(PeerSocket, PathBuf),

    #[display("peerd --bifrost")]
    PeerdBifrost(PeerSocket, PathBuf),

    #[display("channeld")]
    Channeld(ActiveChannelId),

    #[display("routed")]
    Routed,

    #[display("watchd")]
    Watchd,
}

impl Launcher for Daemon {
    type RunError = Error;
    type Config = Config;

    fn bin_name(&self) -> &'static str {
        match self {
            Daemon::Signd => "signd",
            Daemon::PeerdBolt(..) => "peerd",
            Daemon::PeerdBifrost(..) => "peerd",
            Daemon::Channeld(..) => "channeld",
            Daemon::Routed => "routed",
            Daemon::Watchd => "watchd",
        }
    }

    fn cmd_args(&self, cmd: &mut Command) -> Result<(), LauncherError<Self>> {
        cmd.args(std::env::args().skip(1).filter(|arg| !arg.starts_with("--listen")));

        match self {
            Daemon::PeerdBolt(PeerSocket::Listen(node_addr), _) => {
                let ip: IpAddr = node_addr
                    .addr
                    .address()
                    .try_into()
                    .map_err(|_| transport::Error::TorNotSupportedYet)?;
                let port = node_addr.addr.port().ok_or(transport::Error::TorNotSupportedYet)?;
                cmd.args(&["--bolt", "--listen", &ip.to_string(), "--port", &port.to_string()]);
            }
            Daemon::PeerdBolt(PeerSocket::Connect(node_addr), _) => {
                cmd.args(&["--bolt", "--connect", &node_addr.to_string()]);
            }
            Daemon::Channeld(channel_id, ..) => {
                cmd.args(&[channel_id.as_slice32().to_hex()]);
                if channel_id.channel_id().is_some() {
                    cmd.args(&["--reestablish"]);
                }
            }
            _ => { /* No additional configuration is required here */ }
        }

        Ok(())
    }

    fn run_impl(self, config: Config) -> Result<(), Error> {
        match self {
            Daemon::Signd => signd::run(config),
            Daemon::PeerdBolt(socket, key_file) => {
                let local_node = read_node_key_file(&key_file);
                let threaded = config.threaded;
                let config = Config::with(config, peerd::Config { protocol: P2pProtocol::Bolt });
                supervisor::run(
                    config,
                    threaded,
                    FramingProtocol::Brontide,
                    local_node,
                    socket,
                    peerd::runtime::run,
                )
            }
            Daemon::PeerdBifrost(socket, key_file) => {
                let threaded = config.threaded;
                let local_node = read_node_key_file(&key_file);
                let config = Config::with(config, peerd::Config { protocol: P2pProtocol::Bifrost });
                supervisor::run(
                    config,
                    threaded,
                    FramingProtocol::Brontozaur,
                    local_node,
                    socket,
                    peerd::runtime::run,
                )
            }
            Daemon::Channeld(channel_id) => channeld::run(config, channel_id),
            Daemon::Routed => routed::run(config),
            Daemon::Watchd => watchd::run(config),
        }
    }
}

impl Runtime {
    pub(super) fn launch_daemon(
        &self,
        daemon: Daemon,
        config: Config,
    ) -> Result<DaemonHandle<Daemon>, LauncherError<Daemon>> {
        if self.config.threaded {
            daemon.thread_daemon(config)
        } else {
            daemon.exec_daemon()
        }
    }
}

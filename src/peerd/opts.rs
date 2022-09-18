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

use std::net::{IpAddr, Ipv4Addr};

use clap::{ArgGroup, ValueHint};
use internet2::addr::{InetSocketAddr, NodeAddr, NodeId};
use lnp::addr::LnpAddr;
use lnp::p2p;
use lnp::p2p::bifrost::LNP2P_BIFROST_PORT;
use lnp::p2p::bolt::LNP2P_BOLT_PORT;
use microservices::peer::PeerSocket;
use microservices::shell::shell_expand_dir;

use crate::opts::LNP_NODE_KEY_FILE;

/// Lightning peer network connection daemon; part of LNP Node.
///
/// Daemon listens to incoming connections from the lightning network peers (if started
/// with `--listen` argument) or connects to the remote peer (specified with `--connect`
/// argument) and passes all incoming messages into ZMQ messaging socket (controlled with
/// `--msg-socket` argument, defaulting to `msg.rpc` file inside the data directory from
/// `--data-dir`). It also forwards messages from the same socket to the remote peer.
///
/// The daemon is controlled though RPC socket (see `rpc-socket`).
#[derive(Parser, Clone, PartialEq, Eq, Debug)]
#[clap(
    name = "peerd",
    bin_name = "peerd",
    author,
    version,
    group = ArgGroup::new("action").required(true),
)]
pub struct Opts {
    // These params are passed through command-line argument or environment
    // only since they are instance-specific
    /// Start daemon in listening mode binding the provided local address.
    ///
    /// Binds to the specified interface and listens for incoming connections, spawning
    /// a new thread / forking child process for each new incoming client connecting the
    /// opened socket. Whether the child is spawned as a thread or forked as a child
    /// process determined by the presence of `--threaded-daemons` flag.
    ///
    /// If the argument is provided in form of flag, without value, uses `0.0.0.0` as the
    /// bind address.
    #[clap(short = 'L', long, group = "action", value_hint = ValueHint::Hostname)]
    pub listen: Option<Option<IpAddr>>,

    /// Connect to a remote peer with the provided address after start.
    ///
    /// Connects to the specified remote peer. Peer address should be given as either
    /// IPv4, IPv6 or Onion address (v2 or v3); in the former case you will be also
    /// required to provide `--tor` argument.
    #[clap(short = 'C', long, group = "action")]
    pub connect: Option<LnpAddr>,

    /// Customize port used by lightning peer network.
    ///
    /// Optional argument specifying local or remote TCP port to use with the address
    /// given to `--listen` or `--connect` argument.
    #[clap(short, long)]
    pub port: Option<u16>,

    /// Use BOLT lightning network protocol.
    #[clap(long, conflicts_with = "bifrost")]
    pub bolt: bool,

    /// Use Bifrost lightning network protocol.
    #[clap(long, required_unless_present_any = ["connect", "bolt"])]
    pub bifrost: bool,

    /// Node key configuration
    #[clap(flatten)]
    pub key_opts: KeyOpts,

    /// These params can be read also from the configuration file, not just
    /// command-line args or environment variables
    #[clap(flatten)]
    pub shared: crate::opts::Opts,
}

/// Node key configuration
#[derive(Parser, Clone, PartialEq, Eq, Debug)]
pub struct KeyOpts {
    /// Node key file.
    ///
    /// Location for the file containing node private key (unencrypted).
    #[clap(
        short,
        long,
        env = "LNP_NODE_KEY_FILE",
        default_value = LNP_NODE_KEY_FILE,
        value_hint = ValueHint::FilePath
    )]
    pub key_file: String,
}

impl Opts {
    pub fn process(&mut self) {
        if let Some(peer) = self.connect {
            if self.bolt && peer.protocol == p2p::Protocol::Bifrost
                || self.bifrost && peer.protocol == p2p::Protocol::Bolt
            {
                panic!("Provided connection address {} does not match P2P protocol flag", peer);
            }
        }
        self.shared.process();
        self.key_opts.process(&self.shared);
    }

    pub fn protocol(&self) -> p2p::Protocol {
        if self.bolt {
            p2p::Protocol::Bolt
        } else if self.bifrost {
            p2p::Protocol::Bifrost
        } else if let Some(peer) = self.connect {
            peer.protocol
        } else {
            unreachable!()
        }
    }

    pub fn port(&self) -> u16 {
        match self.protocol() {
            p2p::Protocol::Bifrost => LNP2P_BIFROST_PORT,
            p2p::Protocol::Bolt => LNP2P_BOLT_PORT,
        }
    }
}

impl KeyOpts {
    pub fn process(&mut self, shared: &crate::opts::Opts) {
        shell_expand_dir(&mut self.key_file, &shared.data_dir.display().to_string(), &[]);
    }
}

impl Opts {
    pub fn peer_socket(&self, node_id: NodeId) -> PeerSocket {
        if let Some(peer_addr) = self.connect {
            PeerSocket::Connect(peer_addr.node_addr)
        } else if let Some(bind_addr) = self.listen {
            let addr = InetSocketAddr::socket(
                bind_addr.unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED)).into(),
                self.port.unwrap_or(self.port()),
            );
            PeerSocket::Listen(NodeAddr::new(node_id, addr))
        } else {
            unreachable!("Either `connect` or `listen` must be present due to Clap configuration")
        }
    }
}

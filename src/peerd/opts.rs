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

use std::net::{IpAddr, Ipv4Addr};

use clap::{ArgGroup, ValueHint};
use internet2::addr::InetSocketAddr;
use internet2::{FramingProtocol, RemoteNodeAddr, RemoteSocketAddr};

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
    pub connect: Option<RemoteNodeAddr>,

    /// Customize port used by lightning peer network.
    ///
    /// Optional argument specifying local or remote TCP port to use with the address
    /// given to `--listen` or `--connect` argument.
    #[clap(short, long, default_value = "9735")]
    pub port: u16,

    /// Overlay peer communications through different transport protocol.
    #[clap(
        short,
        long,
        default_value = "tcp",
        possible_values = &["tcp", "zmq", "http", "websocket", "smtp"]
    )]
    pub overlay: FramingProtocol,

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
        self.shared.process();
        self.key_opts.process(&self.shared);
    }
}

impl KeyOpts {
    pub fn process(&mut self, shared: &crate::opts::Opts) {
        shared.process_dir(&mut self.key_file);
    }
}

impl From<Opts> for crate::peerd::PeerSocket {
    fn from(opts: Opts) -> Self {
        if let Some(peer_addr) = opts.connect {
            Self::Connect(peer_addr)
        } else if let Some(bind_addr) = opts.listen {
            Self::Listen(match opts.overlay {
                FramingProtocol::FramedRaw => RemoteSocketAddr::Ftcp(InetSocketAddr {
                    address: bind_addr.unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED)).into(),
                    port: opts.port,
                }),
                // TODO: (v2) implement overlay protocols
                _ => unimplemented!(),
            })
        } else {
            unreachable!("Either `connect` or `listen` must be present due to Clap configuration")
        }
    }
}

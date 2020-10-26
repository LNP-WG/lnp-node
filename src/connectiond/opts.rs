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

use clap::{AppSettings, ArgGroup, Clap, ValueHint};
use std::net::IpAddr;
use std::path::PathBuf;

use lnpbp::lnp::{transport::FramingProtocol, NodeAddr};

/// Lightning peer network connection daemon; part of LNP Node
///
/// Daemon listens to incoming connections from the lightning network peers
/// (if started with `--listen` argument) or connects to the remote peer
/// (specified with `--connect` argument) and passes all incoming messages into
/// ZMQ messaging socket (controlled with `--msg-socket` argument, defaulting to
/// `msg.rpc` file inside the data directory from `--data-dir`). It also
/// forwards messages from the same socket to the remote peer.
///
/// The daemon is controlled though ZMQ ctl socket (see `ctl-socket` argument
/// description)
#[derive(Clap, Clone, PartialEq, Eq, Debug)]
#[clap(
    name = "connectiond",
    bin_name = "connectiond",
    author,
    version,
    group = ArgGroup::new("action").required(true),
    setting = AppSettings::ColoredHelp
)]
pub struct Opts {
    // These params are passed through command-line argument or environment
    // only since they are instance-specific
    /// Start daemon in listening mode binding the provided local address
    ///
    /// Binds to the specified interface and listens for incoming connections,
    /// spawning a new thread / forking child process for each new incoming
    /// client connecting the opened socket. Whether the child is spawned as a
    /// thread or forked as a child process determined by the presence of
    /// `--use-threads` flag.
    /// If the argument is provided in form of flag, without value, uses
    /// `0.0.0.0` as the bind address.
    #[clap(short = 'L', long, group = "action", value_hint = ValueHint::Hostname)]
    pub listen: Option<Option<IpAddr>>,

    /// Connect to a remote peer with the provided address after start
    ///
    /// Connects to the specified remote peer. Peer address should be given as
    /// either IPv4, IPv6 or Onion address (v2 or v3); in the former case you
    /// will be also required to provide `--tor` argument.
    #[clap(short = 'C', long, group = "action")]
    pub connect: Option<NodeAddr>,

    /// Customize port used by lightning peer network
    ///
    /// Optional argument specifying local or remote TCP port to use with the
    /// address given to `--listen` or `--connect` argument.
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

    /// Spawn threads instead of forking new processes for incoming connections
    ///
    /// Determines whether incoming connections `--listen` mode should be
    /// forked into a child process or spawned as a threads
    #[clap(
        short = 't',
        long,
        env = "LNP_NODE_USE_THREADS",
        conflicts_with = "connect"
    )]
    pub use_threads: bool,

    /// Node key file
    ///
    /// Location for the file containing node private Secp256k1 key
    /// (unencrypted)
    #[clap(
        short,
        long,
        env = "LNP_NODE_KEY_FILE",
        value_hint = ValueHint::FilePath
    )]
    pub key_file: Option<PathBuf>,

    /// These params can be read also from the configuration file, not just
    /// command-line args or environment variables
    #[clap(flatten)]
    pub shared: crate::opts::Opts,
}

impl Opts {
    pub fn process(&mut self) {
        self.shared.process()
    }
}

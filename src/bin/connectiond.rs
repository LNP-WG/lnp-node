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

//! Main executable for connectiond: lightning peer network connection
//! microservice.
//!
//! Program operations
//! ==================
//!
//! Bootstrapping
//! -------------
//!
//! Since this daemon must operate external P2P TCP socket, and TCP socket can
//! be either connected to the remote or accept remote connections; and we need
//! a daemon per connection, while the incoming TCP socket can't be transferred
//! between processes using IPC, the only option here is to have two special
//! cases of the daemon.
//!
//! The first one will open TCP socket in listening mode and wait for incoming
//! connections, forking on each one of them, passing the accepted TCP socket to
//! the child and continuing on listening to the new connections. (In
//! multi-thread mode, differentiated with `--threaded` argument, instead of
//! forking damon will launch a new thread).
//!
//! The second one will be launched by some control process and then commanded
//! (through command API) to connect to a specific remote TCP socket.
//!
//! These two cases are differentiated by a presence command-line option
//! `--listen` followed by a listening address to bind (IPv4, IPv6, Tor and TCP
//! port number) or `--connect` option followed by a remote address in the same
//! format.
//!
//! Runtime
//! -------
//!
//! The overall program logic thus is the following:
//!
//! In the process starting from `main()`:
//! - Parse cli arguments into a config. There is no config file, since the
//!   daemon can be started only from another control process (`lnpd`) or by
//!   forking itself.
//! - If `--listen` argument is present, start a listening version as described
//!   above and open TCP port in listening mode; wait for incoming connections
//! - If `--connect` argument is present, connect to the remote TCP peer
//!
//! In forked/spawned version:
//! - Acquire connected TCP socket from the parent
//!
//! From here, all actions must be taken by either forked version or by a daemon
//! launched from the control process:
//! - Split TCP socket and related transcoders into reading and writing parts
//! - Create bridge ZMQ PAIR socket
//! - Put both TCP socket reading ZMQ bridge write PAIR parts into a thread
//!   ("bridge")
//! - Open control interface socket
//! - Create run loop in the main thread for polling three ZMQ sockets:
//!   * control interface
//!   * LN P2P messages from intranet
//!   * bridge socket
//!
//! Node key
//! --------
//!
//! Node key, used for node identification and in generation of the encryption
//! keys, is read from the file specified in `--key-file` parameter, or (if the
//! parameter is absent) from `LNP_NODE_KEY_FILE` environment variable.

#![feature(never_type)]

#[macro_use]
extern crate log;
#[macro_use]
extern crate amplify_derive;

use amplify::internet::{InetAddr, InetSocketAddr};
use clap::Clap;
use log::LevelFilter;
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;

use lnpbp::lnp::transport::zmq::SocketLocator;

use lnp_node::constants::*;

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
    name = "LNP connection daemon",
    author = "Dr. Maxim Orlovsky <orlovsky@pandoracore.com>",
    version = "v0.1 (alpha 1)",
    long_version = "v0.1.0-alpha.1"
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
    #[clap(short = 'L', long, group = "bind")]
    pub listen: Option<Option<IpAddr>>,

    /// Connect to a remote peer with the provided address after start
    ///
    /// Connects to the specified remote peer. Peer address should be given as
    /// either IPv4, IPv6 or Onion address (v2 or v3); in the former case you
    /// will be also required to provide `--tor` argument.
    #[clap(short = 'C', long, group = "bind")]
    pub connect: Option<InetAddr>,

    /// Customize port used by lightning peer network
    ///
    /// Optional argument specifying local or remote TCP port to use with the
    /// address given to `--listen` or `--connect` argument.
    #[clap(short, long, default_value = "9735")]
    pub port: u16,

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

    // These params can be read also from the configuration file, not just
    // command-line args or environment variables
    /// Data directory path
    ///
    /// Path to the directory that contains LNP Node data, and where ZMQ RPC
    /// socket files are located
    #[clap(
        short,
        long,
        global = true,
        default_value = LNP_NODE_DATA_DIR,
        env = "LNP_NODE_DATA_DIR"
    )]
    pub data_dir: PathBuf,

    /// Path to the configuration file.
    ///
    /// NB: Command-line options override configuration file values.
    #[clap(
        short,
        long,
        global = true,
        default_value = LNP_NODE_CONFIG,
        env = "LNP_NODE_CONFIG"
    )]
    pub config: String,

    /// Set verbosity level
    ///
    /// Can be used multiple times to increase verbosity
    #[clap(short, long, global = true, parse(from_occurrences))]
    pub verbose: u8,

    /// Node key file
    ///
    /// Location for the file containing node private Secp256k1 key
    /// (unencrypted)
    #[clap(short, long, global = true, env = "LNP_NODE_KEY_FILE")]
    pub key_file: Option<PathBuf>,

    /// Use Tor
    ///
    /// If set, specifies SOCKS5 proxy used for Tor connectivity and directs
    /// all network traffic through Tor network.
    /// Required if `connect` is provided with an Onion address.
    /// If the argument is provided in form of flag, without value, uses
    /// `127.0.0.1:9050` as default Tor proxy address.
    #[clap(
        short = 'T',
        long,
        alias = "tor",
        global = true,
        env = "LNP_NODE_TOR_PROXY"
    )]
    pub tor_proxy: Option<Option<SocketAddr>>,

    /// ZMQ socket name/address to forward all incoming lightning messages
    ///
    /// Internal interface for transmitting P2P lightning network messages.
    /// Defaults to `msg.rpc` file inside `--data-dir` directory, unless
    /// `--use-threads` is specified; in that cases uses in-memory
    /// communication protocol.
    #[clap(short = 'm', long, global = true, env = "LNP_NODE_MSG_SOCKET")]
    pub msg_socket: Option<SocketLocator>,

    /// ZMQ socket name/address for daemon control interface
    ///
    /// Internal interface for control PRC protocol communications
    /// Defaults to `ctl.rpc` file inside `--data-dir` directory, unless
    /// `--use-threads` is specified; in that cases uses in-memory
    /// communication protocol.
    #[clap(short = 'x', long, global = true, env = "LNP_NODE_CTL_SOCKET")]
    pub ctl_socket: Option<SocketLocator>,
}

/// Choses type of service runtime (see `--listen` and `--connect` option
/// details in [`Opts`] structure.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Display)]
pub enum P2pSocket {
    /// The service should listen for incoming connections on a certain
    /// TCP socket, which may be IPv4- or IPv6-based. For Tor hidden services
    /// use IPv4 TCP port proxied as a Tor hidden service in `torrc`.
    #[display("--listen={_0}")]
    Listen(SocketAddr),

    /// The service should connect to the remote peer residing on the provided
    /// address, which may be either IPv4/v6 or Onion V2/v3 address (using
    /// onion hidden services will require
    /// DNS names, due to a censorship vulnerability issues and for avoiding
    /// leaking any information about th elocal node to DNS resolvers, are not
    /// supported.
    #[display("--connect={_0}")]
    Connect(InetSocketAddr),
}

/// Final configuration resulting from data contained in config file environment
/// variables and command-line options. For security reasons node key is kept
/// separately.
#[derive(Clone, PartialEq, Eq, Debug, Display)]
#[display(Debug)]
pub struct SocketConfig {
    /// Socket to use for connecting Lightning peer network
    pub p2p_socket: P2pSocket,

    /// ZMQ RPC socket for transmitting lightning peer network messages
    pub msg_socket: SocketLocator,

    /// ZMQ RPC socket for internal daemon control bus
    pub ctl_socket: SocketLocator,

    /// If set, specifies SOCKS5 proxy used for Tor connectivity. Required if
    /// `p2p_socket` is set to `P2pSocket::Connect` with onion address.
    pub tor_socks5: Option<SocketAddr>,
}

fn main() {
    log::set_max_level(LevelFilter::Trace);
    info!("connectiond: lightning peer network connection microservice");

    Opts::parse();
}

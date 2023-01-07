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

use std::fmt::Debug;
use std::net::SocketAddr;
use std::path::PathBuf;

use clap::ValueHint;
use internet2::addr::ServiceAddr;
use lnp_rpc::LNP_NODE_RPC_ENDPOINT;
use lnpbp::chain::Chain;
use microservices::shell::shell_setup;

const LNP_NODE_CTL_ENDPOINT: &str = "{data_dir}/ctl";

#[cfg(any(target_os = "linux"))]
pub const LNP_NODE_DATA_DIR: &'static str = "~/.lnp_node/{chain}";
#[cfg(any(target_os = "freebsd", target_os = "openbsd", target_os = "netbsd"))]
pub const LNP_NODE_DATA_DIR: &'static str = "~/.lnp_node/{chain}";
#[cfg(target_os = "macos")]
pub const LNP_NODE_DATA_DIR: &str = "~/Library/Application Support/LNP Node/{chain}";
#[cfg(target_os = "windows")]
pub const LNP_NODE_DATA_DIR: &'static str = "~\\AppData\\Local\\LNP Node\\{chain}";
#[cfg(target_os = "ios")]
pub const LNP_NODE_DATA_DIR: &'static str = "~/Documents/{chain}";
#[cfg(target_os = "android")]
pub const LNP_NODE_DATA_DIR: &'static str = "./{chain}";

pub const LNP_NODE_MSG_ENDPOINT: &str = "{data_dir}/msg";

pub const LNP_NODE_CONFIG: &str = "{data_dir}/lnp_node.toml";
pub const LNP_NODE_TOR_PROXY: &str = "127.0.0.1:9050";
pub const LNP_NODE_KEY_FILE: &str = "{data_dir}/node.key";

/// Marker trait for daemon-specific options
pub trait Options: Clone + Eq + Debug {
    /// Daemon-specific configuration extension
    type Conf;

    /// Returns shared part of options
    fn shared(&self) -> &Opts;

    /// Constructs daemon-specific configuration object
    fn config(&self) -> Self::Conf;
}

/// Shared options used by different binaries
#[derive(Parser, Clone, PartialEq, Eq, Debug)]
pub struct Opts {
    /// Set verbosity level.
    ///
    /// Can be used multiple times to increase verbosity.
    #[clap(short, long, global = true, parse(from_occurrences))]
    pub verbose: u8,

    /// Data directory path.
    ///
    /// Path to the directory that contains stored data, and where ZMQ RPC
    /// socket files are located.
    #[clap(
        short,
        long,
        global = true,
        default_value = LNP_NODE_DATA_DIR,
        env = "LNP_NODE_DATA_DIR",
        value_hint = ValueHint::DirPath
    )]
    pub data_dir: PathBuf,

    /// Path for the configuration file.
    ///
    /// NB: Command-line options override configuration file values.
    #[clap(
        short,
        long,
        global = true,
        env = "LNP_NODE_CONFIG",
        value_hint = ValueHint::FilePath
    )]
    pub config: Option<PathBuf>,

    /// Use Tor.
    ///
    /// If set, specifies SOCKS5 proxy used for Tor connectivity and directs all network
    /// traffic through Tor network. If the argument is provided in form of flag, without
    /// value, uses `127.0.0.1:9050` as default Tor proxy address.
    #[clap(
        short = 'T',
        long,
        alias = "tor",
        global = true,
        env = "LNP_NODE_TOR_PROXY",
        value_hint = ValueHint::Hostname
    )]
    pub tor_proxy: Option<Option<SocketAddr>>,

    /// ZMQ socket for peer message bus used to communicate with LNP node peerd
    /// service.
    ///
    /// A user needs to specify this socket usually if it likes to distribute daemons
    /// over different server instances. In this case all daemons within the same node
    /// must use the same socket address.
    ///
    /// Socket can be either TCP address in form of `<ipv4 | ipv6>:<port>` – or a path
    /// to an IPC file.
    ///
    /// Defaults to `msg` file inside `--data-dir` directory.
    #[clap(
        short = 'M',
        long = "msg",
        global = true,
        env = "LNP_NODE_MSG_ENDPOINT",
        default_value = LNP_NODE_MSG_ENDPOINT,
        value_hint = ValueHint::FilePath
    )]
    pub msg_endpoint: ServiceAddr,

    /// ZMQ socket for internal service control bus.
    ///
    /// A user needs to specify this socket usually if it likes to distribute daemons
    /// over different server instances. In this case all daemons within the same node
    /// must use the same socket address.
    ///
    /// Socket can be either TCP address in form of `<ipv4 | ipv6>:<port>` – or a path
    /// to an IPC file.
    ///
    /// Defaults to `ctl` file inside `--data-dir` directory, unless `--threaded-daemons`
    /// is specified; in that cases parameter in-memory communication protocol is used
    /// by default (see ZMQ inproc socket specification).
    #[clap(
        short = 'X',
        long = "ctl",
        global = true,
        env = "LNP_NODE_CTL_ENDPOINT",
        default_value = LNP_NODE_CTL_ENDPOINT,
        value_hint = ValueHint::FilePath
    )]
    pub ctl_endpoint: ServiceAddr,

    /// ZMQ socket for LNP Node client-server RPC API.
    ///
    /// Socket can be either TCP address in form of `<ipv4 | ipv6>:<port>` – or a path
    /// to an IPC file.
    #[clap(
        short = 'R',
        long = "rpc",
        global = true,
        default_value = LNP_NODE_RPC_ENDPOINT,
        env = "LNP_NODE_RPC_ENDPOINT"
    )]
    pub rpc_endpoint: ServiceAddr,

    /// Blockchain to use
    #[clap(
        short = 'n',
        long,
        global = true,
        alias = "network",
        default_value = "signet",
        env = "LNP_NODE_NETWORK"
    )]
    pub chain: Chain,

    /// Electrum server to use.
    #[clap(
        long,
        global = true,
        default_value("electrum.blockstream.info"),
        env = "LNP_NODE_ELECTRUM_SERVER",
        value_hint = ValueHint::Hostname
    )]
    pub electrum_server: String,

    /// Customize Electrum server port number. By default the wallet will use port
    /// matching the selected network.
    #[clap(long, global = true, env = "LNP_NODE_ELECTRUM_PORT")]
    pub electrum_port: Option<u16>,

    /// Spawn daemons as threads and not processes
    #[clap(short = 't', long = "threaded")]
    pub threaded_daemons: bool,
}

impl Opts {
    pub fn process(&mut self) {
        shell_setup(
            self.verbose,
            [&mut self.msg_endpoint, &mut self.ctl_endpoint, &mut self.rpc_endpoint],
            &mut self.data_dir,
            &[("{chain}", self.chain.to_string())],
        );
    }
}

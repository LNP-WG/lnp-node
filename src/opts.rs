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

use std::fs;
use std::net::SocketAddr;
use std::path::PathBuf;

use clap::ValueHint;
use lnp_rpc::LNP_NODE_RPC_SOCKET;
use lnpbp::chain::Chain;
use microservices::shell::LogLevel;

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

pub const LNP_NODE_MSG_SOCKET: &str = "{data_dir}/msg";
pub const LNP_NODE_CTL_SOCKET: &str = "{data_dir}/ctl";

pub const LNP_NODE_CONFIG: &str = "{data_dir}/lnp_node.toml";
pub const LNP_NODE_TOR_PROXY: &str = "127.0.0.1:9050";
pub const LNP_NODE_KEY_FILE: &str = "{data_dir}/node.key";

pub const LNP_NODE_MASTER_KEY_FILE: &str = "master.key";
pub const LNP_NODE_FUNDING_WALLET: &str = "funding.wallet";

/// Shared options used by different binaries
#[derive(Parser, Clone, PartialEq, Eq, Debug)]
pub struct Opts {
    /// <[_]<[_]>::into_vec(box [$($x),+]).into_iter().flatten()
    /// are located.
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

    /// Set verbosity level.
    ///
    /// Can be used multiple times to increase verbosity.
    #[clap(short, long, global = true, parse(from_occurrences))]
    pub verbose: u8,

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

    /// ZMQ socket for internal message bus.
    ///
    /// A user needs to specify this socket usually if it likes to distribute daemons
    /// over different server instances. In this case all daemons within the same node
    /// must use the same socket address.
    ///
    /// Socket can be either TCP address in form of `<ipv4 | ipv6>:<port>` – or a path
    /// to an IPC file.
    ///
    /// Defaults to `msg` file inside `--data-dir` directory, unless `--threaded-daemons`
    /// is specified; in that cases uses in-memory communication protocol.
    #[clap(long = "msg", global = true, env = "LNP_NODE_MSG_SOCKET", value_hint = ValueHint::FilePath)]
    pub msg_socket: Option<String>,

    /// ZMQ socket for internal service bus.
    ///
    /// A user needs to specify this socket usually if it likes to distribute daemons
    /// over different server instances. In this case all daemons within the same node
    /// must use the same socket address.
    ///
    /// Socket can be either TCP address in form of `<ipv4 | ipv6>:<port>` – or a path
    /// to an IPC file.
    ///
    /// Defaults to `ctl` file inside `--data-dir` directory, unless `--threaded-daemons`
    /// is specified; in that cases uses in-memory communication protocol.
    #[clap(long = "ctl", global = true, env = "LNP_NODE_CTL_SOCKET", value_hint = ValueHint::FilePath)]
    pub ctl_socket: Option<String>,

    /// ZMQ socket for connecting daemon RPC interface.
    ///
    /// Socket can be either TCP address in form of `<ipv4 | ipv6>:<port>` – or a path
    /// to an IPC file.
    ///
    /// Defaults to `127.0.0.1:62962`.
    #[clap(
        short = 'r',
        long = "rpc",
        global = true,
        default_value = LNP_NODE_RPC_SOCKET,
        env = "LNP_NODE_RPC_SOCKET"
    )]
    pub rpc_socket: String,

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
        default_value("pandora.network"),
        env = "LNP_NODE_ELECTRUM_SERVER",
        value_hint = ValueHint::Hostname
    )]
    pub electrum_server: String,

    /// Customize Electrum server port number. By default the wallet will use port
    /// matching the selected network.
    #[clap(long, global = true, env = "LNP_NODE_ELECTRUM_PORT")]
    pub electrum_port: Option<u16>,

    /// Spawn daemons as threads and not processes
    #[clap(long)]
    pub threaded_daemons: bool,
}

impl Opts {
    pub fn process(&mut self) {
        LogLevel::from_verbosity_flag_count(self.verbose).apply();
        let me = self.clone();

        let mut data_dir = me.data_dir.display().to_string();
        data_dir = data_dir.replace("{chain}", &self.chain.to_string());
        self.data_dir = PathBuf::from(shellexpand::tilde(&data_dir).to_string());
        fs::create_dir_all(&self.data_dir).unwrap_or_else(|_| {
            panic!("Unable to access data directory '{}'", &self.data_dir.display())
        });

        for s in self.msg_socket.iter_mut().chain(self.ctl_socket.iter_mut()) {
            me.process_dir(s);
        }
    }

    pub fn process_dir(&self, path: &mut String) {
        process_dir(path, &self.data_dir.display().to_string());
    }
}

pub fn process_dir(path: &mut String, data_dir: &str) {
    *path = path.replace("{data_dir}", data_dir);
    *path = shellexpand::tilde(path).to_string();
}

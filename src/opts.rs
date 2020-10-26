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

use clap::{Clap, ValueHint};
use std::net::SocketAddr;
use std::path::PathBuf;

use lnpbp::lnp::NodeLocator;

pub const LNP_NODE_CONFIG: &'static str = "{data_dir}/lnpd.toml";
#[cfg(any(target_os = "linux"))]
pub const LNP_NODE_DATA_DIR: &'static str = "~/.lnp_node/";
#[cfg(any(target_os = "freebsd", target_os = "openbsd", target_os = "netbsd"))]
pub const LNP_NODE_DATA_DIR: &'static str = "~/.lnp_node/";
#[cfg(target_os = "macos")]
pub const LNP_NODE_DATA_DIR: &'static str =
    "~/Library/Application Support/LNP Node/";
#[cfg(target_os = "windows")]
pub const LNP_NODE_DATA_DIR: &'static str = "~\\AppData\\Local\\LNP Node\\";
#[cfg(target_os = "ios")]
pub const LNP_NODE_DATA_DIR: &'static str = "~/Documents/";
#[cfg(target_os = "android")]
pub const LNP_NODE_DATA_DIR: &'static str = ".";

pub const LNP_NODE_MSG_SOCKET_NAME: &'static str = "lnpz:{data_dir}/msg.rpc";
pub const LNP_NODE_CTL_SOCKET_NAME: &'static str = "lnpz:{data_dir}/ctl.rpc";

pub const LNP_NODE_BIND: &'static str = "0.0.0.0:20202";
pub const LNP_NODE_TOR_PROXY: &'static str = "127.0.0.1:9050";

/// Shared options used by different binaries
#[derive(Clap, Clone, PartialEq, Eq, Debug)]
pub struct Opts {
    /// Data directory path
    ///
    /// Path to the directory that contains LNP Node data, and where ZMQ RPC
    /// socket files are located
    #[clap(
        short,
        long,
        global = true,
        default_value = LNP_NODE_DATA_DIR,
        env = "LNP_NODE_DATA_DIR",
        value_hint = ValueHint::DirPath
    )]
    pub data_dir: PathBuf,

    /// Path to the configuration file.
    ///
    /// NB: Command-line options override configuration file values.
    #[clap(
        short,
        long,
        global = true,
        env = "LNP_NODE_CONFIG",
        value_hint = ValueHint::FilePath
    )]
    pub config: Option<String>,

    /// Set verbosity level
    ///
    /// Can be used multiple times to increase verbosity
    #[clap(short, long, global = true, parse(from_occurrences))]
    pub verbose: u8,

    /// Use Tor
    ///
    /// If set, specifies SOCKS5 proxy used for Tor connectivity and directs
    /// all network traffic through Tor network.
    /// If the argument is provided in form of flag, without value, uses
    /// `127.0.0.1:9050` as default Tor proxy address.
    #[clap(
        short = 'T',
        long,
        alias = "tor",
        global = true,
        env = "LNP_NODE_TOR_PROXY",
        value_hint = ValueHint::Hostname
    )]
    pub tor_proxy: Option<Option<SocketAddr>>,

    /// ZMQ socket name/address to forward all incoming lightning messages
    ///
    /// Internal interface for transmitting P2P lightning network messages.
    /// Defaults to `msg.rpc` file inside `--data-dir` directory, unless
    /// `--use-threads` is specified; in that cases uses in-memory
    /// communication protocol.
    #[clap(
        short = 'm',
        long,
        global = true,
        env = "LNP_NODE_MSG_SOCKET",
        value_hint = ValueHint::FilePath,
        default_value = LNP_NODE_MSG_SOCKET_NAME
    )]
    pub msg_socket: NodeLocator,

    /// ZMQ socket name/address for daemon control interface
    ///
    /// Internal interface for control PRC protocol communications
    /// Defaults to `ctl.rpc` file inside `--data-dir` directory, unless
    /// `--use-threads` is specified; in that cases uses in-memory
    /// communication protocol.
    #[clap(
        short = 'x',
        long,
        global = true,
        env = "LNP_NODE_CTL_SOCKET",
        value_hint = ValueHint::FilePath,
        default_value = LNP_NODE_CTL_SOCKET_NAME
    )]
    pub ctl_socket: NodeLocator,
}

impl Opts {
    pub fn process(&mut self) {
        for s in vec![&mut self.msg_socket, &mut self.ctl_socket] {
            match s {
                NodeLocator::ZmqIpc(path, ..) | NodeLocator::Posix(path) => {
                    *path = path
                        .replace("{data_dir}", &self.data_dir.to_string_lossy())
                }
                _ => unimplemented!(),
            }
        }
    }
}

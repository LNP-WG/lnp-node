// Lightning network protocol (LNP) daemon suite
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


use std::str::FromStr;
use clap::Clap;

use lnpbp::lnp::NodeAddr;

use crate::msgbus::constants::*;


#[derive(Clap)]
#[clap(
    name = "lnp-cli",
    version = "0.0.1",
    author = "Dr Maxim Orlovsky <orlovsky@pandoracore.com>",
    about =  "LNP node command-line interface; part of Lightning network protocol suite"
)]
pub struct Opts {
    /// Path and name of the configuration file
    #[clap(global = true, short = "c", long = "config", default_value = "./cli.toml")]
    pub config: String,

    /// Sets verbosity level
    #[clap(global = true, short = "v", long = "verbose", min_values=0, max_values=4, parse(from_occurrences))]
    pub verbose: i32,

    /// IPC connection string for wired daemon API
    #[clap(global = true, short = "w", long = "wired-api", default_value = MSGBUS_PEER_API_ADDR, env="LNP_CLI_WIRED_API_ADDR")]
    wired_api_socket_str: String,

    /// IPC connection string for wired daemon push notifications on perr status updates
    #[clap(global = true, short = "W", long = "wired-push", default_value = MSGBUS_PEER_PUSH_ADDR, env="LNP_CLI_WIRED_PUSH_ADDR")]
    wired_push_socket_str: String,

    #[clap(subcommand)]
    command: Command
}

#[derive(Clap)]
pub enum Command {
    /// Sends command to a wired daemon to connect to the new peer
    Connect {
        /// Peer address string, in format `<node_id>@<node_inet_addr>[:<port>]`,
        /// where <node_inet_addr> may be IPv4, IPv6 or TORv3 address
        #[clap(parse(try_from_str))]
        addr: NodeAddr
    },

    /// Lists all connected peers
    ListConnections,
}


// We need config structure since not all of the parameters can be specified
// via environment and command-line arguments. Thus we need a config file and
// default set of configuration
#[derive(Clone, PartialEq, Eq, Debug, Display)]
#[display_from(Debug)]
pub struct Config {
    pub verbose: u8,
    pub msgbus_peer_api_addr: String,
    pub msgbus_peer_sub_addr: String,
}

impl From<Opts> for Config {
    fn from(opts: Opts) -> Self {
        Self {
            verbose: opts.verbose as u8,
            msgbus_peer_api_addr: opts.wired_api_socket_str,
            msgbus_peer_sub_addr: opts.wired_push_socket_str,
            ..Config::default()
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            verbose: 0,
            msgbus_peer_api_addr: MSGBUS_PEER_API_ADDR.to_string(),
            msgbus_peer_sub_addr: MSGBUS_PEER_PUSH_ADDR.to_string()
        }
    }
}
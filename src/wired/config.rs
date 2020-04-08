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


use std::net::SocketAddr;
use clap::Clap;

use lnpbp::internet::{InetSocketAddr, InetAddr};
use lnpbp::lnp::NodeAddr;

use crate::msgbus::constants::*;

const MONITOR_ADDR_DEFAULT: &str = "0.0.0.0:9666";

#[derive(Clap)]
#[clap(
    name = "wired",
    version = "0.0.1",
    author = "Dr Maxim Orlovsky <orlovsky@pandoracore.com>",
    about =  "LNP wired: Lightning wire P2P daemon; part of Lightning network protocol suite"
)]
pub struct Opts {
    /// Path and name of the configuration file
    #[clap(short = "c", long = "config", default_value = "wired.toml")]
    pub config: String,

    /// Sets verbosity level; can be used multiple times to increase verbosity
    #[clap(global = true, short = "v", long = "verbose", min_values = 0, max_values = 4, parse(from_occurrences))]
    pub verbose: u8,

    /// IPv4, IPv6 or Tor address to listen for incoming connections from LN peers
    #[clap(short = "i", long = "inet-addr", default_value = "0.0.0.0", env="LNP_WIRED_INET_ADDR",
           parse(try_from_str))]
    address: InetAddr,

    /// Use custom port to listen for incoming connections from LN peers
    #[clap(short = "p", long = "port", default_value = "9735", env="LNP_WIRED_PORT")]
    port: u16,

    /// Address for Prometheus monitoring information exporter
    #[clap(short = "m", long = "monitor", default_value = MONITOR_ADDR_DEFAULT, env="LNP_WIRED_MONITOR",
           parse(try_from_str))]
    monitor: SocketAddr,

    // TODO: Use connect argument for connecting to the nodes after the launch
    /// Nodes to connect at after the launch
    /// (in form of `<node_id>@<inet_addr>[:<port>]`,
    /// where <inet_addr> can be IPv4, IPv6 or TORv3 internet address)
    #[clap(short = "C", long = "connect", min_values=0, env="LNP_WIRED_CONNECT")]
    connect: Vec<NodeAddr>,
}


// We need config structure since not all of the parameters can be specified
// via environment and command-line arguments. Thus we need a config file and
// default set of configuration
#[derive(Clone, PartialEq, Eq, Debug, Display)]
#[display_from(Debug)]
pub struct Config {
    pub verbose: u8,
    pub lnp2p_addr: InetSocketAddr,
    pub monitor_addr: SocketAddr,
    pub msgbus_peer_api_addr: String,
    pub msgbus_peer_push_addr: String,
}

impl From<Opts> for Config {
    fn from(opts: Opts) -> Self {
        Self {
            verbose: opts.verbose,
            lnp2p_addr: InetSocketAddr::new(opts.address, opts.port),
            monitor_addr: opts.monitor,
            ..Config::default()
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            verbose: 0,
            lnp2p_addr: InetSocketAddr::default(),
            monitor_addr: MONITOR_ADDR_DEFAULT.parse().expect("Constant default value parse fail"),
            msgbus_peer_api_addr: MSGBUS_PEER_API_ADDR.to_string(),
            msgbus_peer_push_addr: MSGBUS_PEER_PUSH_ADDR.to_string()
        }
    }
}
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
use std::convert::TryInto;
use std::str::FromStr;
use std::fmt;

use lnpbp::internet::InetSocketAddr;
use lnpbp::lnp::NodeAddr;

use crate::msgbus::constants::*;
use configure_me::parse_arg::ParseArgFromStr;

const MONITOR_ADDR_DEFAULT: &str = "0.0.0.0:9666";

mod internal {
    #![allow(unused)]
    include!(concat!(env!("OUT_DIR"), "/configure_me_config.rs"));
}

#[derive(Default, Deserialize)]
pub struct Connect(Vec<NodeAddr>);

impl Connect {
    fn merge(&mut self, other: Self) {
        self.0.extend(other.0);
    }
}

impl FromStr for Connect {
    type Err = <NodeAddr as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        NodeAddr::from_str(s).map(|conn| Connect(vec![conn]))
    }
}

impl ParseArgFromStr for Connect {
    fn describe_type<W: fmt::Write>(mut writer: W) -> fmt::Result {
        write!(writer, "node address in form of <node_id>@<inet_addr>[:<port>] where <inet_addr> can be IPv4, IPv6 or TORv3 internet address")
    }
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

impl Config {
    /// Gathers the configuration from arguments, env vars and config files, exits on error.
    pub fn gather_or_exit() -> Self {
        use self::internal::ResultExt;

        let (config, _) = internal::Config::including_optional_config_files(std::iter::empty::<&str>())
            .unwrap_or_exit();

        Config {
            verbose: config.verbose.try_into().unwrap_or_else(|_| 4),
            lnp2p_addr: InetSocketAddr::new(config.inet_addr, config.port),
            monitor_addr: config.monitor,
            msgbus_peer_api_addr: MSGBUS_PEER_API_ADDR.to_string(),
            msgbus_peer_push_addr: MSGBUS_PEER_PUSH_ADDR.to_string()
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            verbose: 0,
            lnp2p_addr: InetSocketAddr::default(),
            monitor_addr: MONITOR_ADDR_DEFAULT.parse().expect("Failed to parse constant MONITOR_ADDR_DEFAULT"),
            msgbus_peer_api_addr: MSGBUS_PEER_API_ADDR.to_string(),
            msgbus_peer_push_addr: MSGBUS_PEER_PUSH_ADDR.to_string()
        }
    }
}

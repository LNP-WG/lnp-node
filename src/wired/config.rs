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

use lnpbp::internet::InetSocketAddr;

use crate::msgbus::constants::*;

mod internal {
    #![allow(unused)]
    include!(concat!(env!("OUT_DIR"), "/configure_me_config.rs"));
}

// Needs to be fn instead of const due to SocketAddr::new() not being const
fn monitor_addr_default() -> SocketAddr {
    SocketAddr::new([0, 0, 0, 0].into(), 9666)
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
            monitor_addr: monitor_addr_default(),
            msgbus_peer_api_addr: MSGBUS_PEER_API_ADDR.to_string(),
            msgbus_peer_push_addr: MSGBUS_PEER_PUSH_ADDR.to_string()
        }
    }
}
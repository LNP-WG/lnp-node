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


use lnpbp::common::internet::InetSocketAddr;

use crate::wired::config::Config as MainConfig;
use crate::constants::*;

#[derive(Clone, PartialEq, Eq, Debug, Display)]
#[display_from(Debug)]
pub struct Config {
    pub lnp2p_addr: InetSocketAddr,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            lnp2p_addr: LNP2P_ADDR.parse()
                .expect("Parsing of constant value `LNP2P_ADDR` failed"),
        }
    }
}

impl From<MainConfig> for Config {
    fn from(config: MainConfig) -> Self {
        Config {
            lnp2p_addr: config.lnp2p_addr,
        }
    }
}

// Lightning network protocol (LNP) daemon suit
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

use super::{p2p, api};

#[derive(Clone, PartialEq, Eq, Debug, Display)]
#[display_from(Debug)]
pub struct Config {
    pub lnp2p_addr: String,
    pub publish_addr: String,
    pub subscribe_addr: String,
}

impl Default for Config {
    fn default() -> Self {
        let p2p_config = p2p::Config::default();
        let api_config = api::Config::default();
        Self {
            lnp2p_addr: p2p_config.lnp2p_addr,
            publish_addr: p2p_config.msgbus_addr,
            subscribe_addr: api_config.socket_addr,
        }
    }
}

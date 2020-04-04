// Lightning network protocol (LNP) daemon
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


use crate::config::Config as MainConfig;
use crate::constants::*;

#[derive(Clone, PartialEq, Eq, Debug, Display)]
#[display_from(Debug)]
pub struct Config {
    pub socket: String
}

impl Default for Config {
    fn default() -> Self {
        Self {
            socket: MONITOR_ADDR.to_string()
        }
    }
}

impl From<MainConfig> for Config {
    fn from(config: MainConfig) -> Self {
        Config {
            socket: config.monitor_socket
        }
    }
}

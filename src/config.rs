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

use crate::{monitor, constants::*};

#[derive(Clone, PartialEq, Eq, Debug, Display)]
#[display_from(Debug)]
pub struct Config {
    pub monitor_socket: String,
    pub db_state_url: String,
}

impl Default for Config {
    fn default() -> Self {
        let monitor_config = monitor::Config::default();
        Self {
            monitor_socket: monitor_config.socket,
            db_state_url: STATE_DB_PATH.to_string(),
        }
    }
}

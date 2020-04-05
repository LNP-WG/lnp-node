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

use crate::{peer, monitor, constants::*};

#[derive(Clone, PartialEq, Eq, Debug, Display)]
#[display_from(Debug)]
pub struct Config {
    pub peer_socket: String,
    pub monitor_socket: String,
    pub responder_socket: String,
    pub publisher_socket: String,
    pub db_state_url: String,

}

impl Default for Config {
    fn default() -> Self {
        let peer_config = peer::Config::default();
        let monitor_config = monitor::Config::default();
        let api_config = api::Config::default();
        Self {
            peer_socket: peer_config.socket,
            monitor_socket: monitor_config.socket,
            responder_socket: api_config.responder_socket,
            publisher_socket: api_config.publisher_socket,
            db_state_url: STATE_DB_PATH.to_string(),
        }
    }
}

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

pub const MSGBUS_PEER_API: &str = "ipc:///tmp/lnp/peer/";
pub const MSGBUS_PEER_P2P_NOTIFY: &str = "ipc:///tmp/lnp/peer/notify";
pub const LNP2P_ADDR: &str = "0.0.0.0:9735";
pub const LNP2P_PORT: u16 = 9735;
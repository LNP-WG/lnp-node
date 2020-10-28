// LNP Node: node running lightning network protocol and generalized lightning
// channels.
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

use lnpbp::lnp::application::ChannelId;
use lnpbp::lnp::NodeEndpoint;

/// Identifiers of daemons participating in LNP Node
pub enum DaemonId {
    Lnpd,
    Gossip,
    Routing,
    Connection(String),
    Channel(ChannelId),
    Foreign(String),
}

impl AsRef<[u8]> for DaemonId {
    fn as_ref(&self) -> &[u8] {
        match self {
            DaemonId::Lnpd => "lnpd".as_bytes(),
            DaemonId::Gossip => "gossipd".as_bytes(),
            DaemonId::Routing => "routed".as_bytes(),
            DaemonId::Connection(endpoint) => endpoint.as_ref(),
            DaemonId::Channel(channel_id) => channel_id.as_ref(),
            DaemonId::Foreign(name) => name.as_bytes(),
        }
    }
}

impl From<Vec<u8>> for DaemonId {
    fn from(vec: Vec<u8>) -> Self {
        match vec.as_slice() {
            v if v == "lnpd".as_bytes() => DaemonId::Lnpd,
            v if v == "gossipd".as_bytes() => DaemonId::Gossip,
            v if v == "routed".as_bytes() => DaemonId::Routing,
            v => {
                let s = String::from_utf8_lossy(v).to_string();
                if NodeEndpoint::from_str(&s).is_ok() {
                    DaemonId::Connection(s)
                } else if v.len() == 32 {
                    let mut hash = [0u8; 32];
                    hash.copy_from_slice(v);
                    DaemonId::Channel(ChannelId::from(hash))
                } else {
                    DaemonId::Foreign(s)
                }
            }
        }
    }
}

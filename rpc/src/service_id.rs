// LNP Node: node running lightning network protocol and generalized lightning
// channels.
// Written in 2020-2024 by
//     Dr. Maxim Orlovsky <orlovsky@lnp-bp.org>
//
// To the extent possible under law, the author(s) have dedicated all
// copyright and related and neighboring rights to this software to
// the public domain worldwide. This software is distributed without
// any warranty.
//
// You should have received a copy of the MIT License along with this software.
// If not, see <https://opensource.org/licenses/MIT>.

use std::str::FromStr;

use internet2::addr::NodeId;
use lnp::p2p::bifrost::BifrostApp;
use lnp::p2p::bolt::{ChannelId, TempChannelId};
use microservices::esb::{self, ClientId, ServiceName};
use strict_encoding::{strict_deserialize, strict_serialize};

/// Identifiers of daemons participating in LNP Node
#[derive(Clone, PartialEq, Eq, Hash, Debug, Display, From, StrictEncode, StrictDecode)]
pub enum ServiceId {
    #[display("loopback")]
    #[strict_encoding(value = 0)]
    Loopback,

    #[display("lnpd")]
    #[strict_encoding(value = 0x20)]
    LnpBroker,

    #[display("watchd")]
    #[strict_encoding(value = 0x27)]
    Watch,

    #[display("routed")]
    #[strict_encoding(value = 0x26)]
    Router,

    #[display("peerd<bolt, {0}>")]
    #[strict_encoding(value = 0x21)]
    PeerBolt(NodeId), // TODO: Switch on just NodeId's

    #[display("peerd<bifrost, {0}>")]
    #[strict_encoding(value = 0x22)]
    PeerBifrost(NodeId),

    #[display("channel<{0:#x}>")]
    #[from]
    #[from(TempChannelId)]
    #[strict_encoding(value = 0x23)]
    Channel(ChannelId),

    #[display("client<{0}>")]
    #[strict_encoding(value = 2)]
    Client(ClientId),

    #[display("signer")]
    #[strict_encoding(value = 0x1F)]
    Signer,

    #[display("msgapp<{0}>")]
    #[strict_encoding(value = 0x25)]
    MsgApp(BifrostApp),

    #[display("chapp<{0}>")]
    #[strict_encoding(value = 0x24)]
    ChannelApp(BifrostApp),

    #[display("other<{0}>")]
    #[strict_encoding(value = 0xFF)]
    Other(ServiceName),
}

impl ServiceId {
    pub fn router() -> ServiceId { ServiceId::LnpBroker }

    pub fn client() -> ServiceId {
        use bitcoin::secp256k1::rand;
        ServiceId::Client(rand::random())
    }

    pub fn to_remote_id(&self) -> Option<NodeId> {
        match self {
            ServiceId::PeerBolt(node_addr) => Some(node_addr.clone()),
            _ => None,
        }
    }
}

impl esb::ServiceAddress for ServiceId {}

impl From<ServiceId> for Vec<u8> {
    fn from(daemon_id: ServiceId) -> Self {
        strict_serialize(&daemon_id).expect("Memory-based encoding does not fail")
    }
}

impl From<Vec<u8>> for ServiceId {
    fn from(vec: Vec<u8>) -> Self {
        strict_deserialize(&vec).unwrap_or_else(|_| {
            ServiceId::Other(
                ServiceName::from_str(&String::from_utf8_lossy(&vec))
                    .expect("ClientName conversion never fails"),
            )
        })
    }
}

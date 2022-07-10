// LNP Node: node running lightning network protocol and generalized lightning
// channels.
// Written in 2020-2022 by
//     Dr. Maxim Orlovsky <orlovsky@lnp-bp.org>
//
// To the extent possible under law, the author(s) have dedicated all
// copyright and related and neighboring rights to this software to
// the public domain worldwide. This software is distributed without
// any warranty.
//
// You should have received a copy of the MIT License along with this software.
// If not, see <https://opensource.org/licenses/MIT>.

use std::fmt::{self, Display, Formatter};
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;

use internet2::addr::{AddrParseError, PartialSocketAddr};
use lnp::p2p;
use lnp::p2p::bifrost::LNP2P_BIFROST_PORT;
use lnp::p2p::bolt::LNP2P_BOLT_PORT;

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
#[derive(NetworkEncode, NetworkDecode)]
pub struct ListenAddr {
    pub protocol: p2p::Protocol,
    pub socket_addr: SocketAddr,
}

impl ListenAddr {
    /// Construct BOLT-compatible listening address.
    pub fn bolt(ip_addr: IpAddr, port: Option<u16>) -> ListenAddr {
        ListenAddr {
            protocol: p2p::Protocol::Bolt,
            socket_addr: SocketAddr::new(ip_addr, port.unwrap_or(LNP2P_BOLT_PORT)),
        }
    }

    /// Construct Bifrost-compatible listening address.
    pub fn bifrost(ip_addr: IpAddr, port: Option<u16>) -> ListenAddr {
        ListenAddr {
            protocol: p2p::Protocol::Bifrost,
            socket_addr: SocketAddr::new(ip_addr, port.unwrap_or(LNP2P_BIFROST_PORT)),
        }
    }
}

impl Display for ListenAddr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}://{}", self.protocol, self.socket_addr.ip())?;
        if self.protocol.default_port() != self.socket_addr.port() {
            write!(f, ":{}", self.socket_addr.port())?;
        }
        Ok(())
    }
}

impl FromStr for ListenAddr {
    type Err = AddrParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut split = s.split("://");
        match (split.next().map(str::to_lowercase).as_deref(), split.next(), split.next()) {
            (Some("bolt"), Some(addr), None) => {
                let socket_addr = PartialSocketAddr::from_str(addr)?;
                let socket_addr: SocketAddr = socket_addr
                    .inet_socket(LNP2P_BOLT_PORT)
                    .try_into()
                    // TODO: Remove mapping
                    .map_err(|_| AddrParseError::NeedsTorFeature)?;
                Ok(ListenAddr { protocol: p2p::Protocol::Bolt, socket_addr })
            }
            (Some("bifrost"), Some(addr), None) => {
                let socket_addr = PartialSocketAddr::from_str(addr)?;
                let socket_addr: SocketAddr = socket_addr.inet_socket(LNP2P_BIFROST_PORT).try_into()
                        // TODO: Remove mapping
                        .map_err(|_| AddrParseError::NeedsTorFeature)?;
                Ok(ListenAddr { protocol: p2p::Protocol::Bifrost, socket_addr })
            }
            (Some(unknown), ..) => Err(AddrParseError::UnknownProtocolError(unknown.to_owned())),
            _ => Err(AddrParseError::WrongAddrFormat(s.to_owned())),
        }
    }
}

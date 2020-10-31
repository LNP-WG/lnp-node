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

use amplify::Wrapper;
use std::convert::{TryFrom, TryInto};
use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

use lnpbp::bitcoin::hashes::hex::{self, ToHex};
use lnpbp::lnp::{
    zmqsocket, AddrError, ChannelId, LocalSocketAddr, NodeAddr,
    PartialNodeAddr, RemoteNodeAddr, RemoteSocketAddr, TempChannelId, ZmqType,
};
use lnpbp::strict_encoding::{strict_decode, strict_encode};
use lnpbp_services::esb;
#[cfg(feature = "node")]
use lnpbp_services::node::TryService;

use crate::rpc::{Request, ServiceBus};
use crate::Config;
#[cfg(feature = "node")]
use crate::Error;

#[derive(
    Wrapper,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Debug,
    From,
    Default,
    StrictEncode,
    StrictDecode,
)]
pub struct ClientName([u8; 32]);

impl Display for ClientName {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(
                f,
                "{}..{}",
                self.0[..4].to_hex(),
                self.0[(self.0.len() - 4)..].to_hex()
            )
        } else {
            f.write_str(&String::from_utf8_lossy(&self.0))
        }
    }
}

impl FromStr for ClientName {
    type Err = hex::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() > 32 {
            let mut me = Self::default();
            me.0.copy_from_slice(&s.as_bytes()[0..32]);
            Ok(me)
        } else {
            let mut me = Self::default();
            me.0[0..s.len()].copy_from_slice(s.as_bytes());
            Ok(me)
        }
    }
}

/// Identifiers of daemons participating in LNP Node
#[derive(
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Debug,
    Display,
    From,
    StrictEncode,
    StrictDecode,
)]
pub enum ServiceId {
    #[display("loopback")]
    Loopback,

    #[display("lnpd")]
    Lnpd,

    #[display("gossipd")]
    Gossip,

    #[display("routed")]
    Routing,

    #[display("connectiond<{_0}>")]
    Connection(RemoteSocketAddr),

    #[display("channel<{_0:#x}>")]
    #[from]
    #[from(TempChannelId)]
    Channel(ChannelId),

    #[display("client<{_0}>")]
    Client(ClientName),
}

impl ServiceId {
    pub fn router() -> ServiceId {
        ServiceId::Lnpd
    }
}

impl esb::ServiceAddress for ServiceId {}

impl From<ServiceId> for Vec<u8> {
    fn from(daemon_id: ServiceId) -> Self {
        strict_encode(&daemon_id).expect("Memory-based encoding does not fail")
    }
}

impl From<Vec<u8>> for ServiceId {
    fn from(vec: Vec<u8>) -> Self {
        strict_decode(&vec).unwrap_or_else(|_| {
            ServiceId::Client(
                ClientName::from_str(&String::from_utf8_lossy(&vec))
                    .expect("ClientName conversion never fails"),
            )
        })
    }
}

/// Strictly-formatted peer id type for interoperable and transferrable node id
/// storage. Convertible from and to [`RemoteNodeAddr`], [`RemoteSocketAddr`],
/// [`NodeAddr`] and from [`PartialNodeAddr`]
// TODO: Move into LNP/BP Core library
#[derive(
    Wrapper,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Debug,
    Display,
    From,
    StrictEncode,
    StrictDecode,
)]
#[display("{repr}")]
pub struct PeerId {
    #[from(PartialNodeAddr)]
    #[from(RemoteNodeAddr)]
    #[from(RemoteSocketAddr)]
    #[from(LocalSocketAddr)]
    #[from(NodeAddr)]
    repr: String,
}

impl TryFrom<PeerId> for RemoteNodeAddr {
    type Error = AddrError;

    fn try_from(peer_id: PeerId) -> Result<Self, Self::Error> {
        RemoteNodeAddr::from_str(peer_id.as_inner())
    }
}

impl TryFrom<PeerId> for NodeAddr {
    type Error = AddrError;

    fn try_from(peer_id: PeerId) -> Result<Self, Self::Error> {
        NodeAddr::from_str(peer_id.as_inner())
    }
}

impl TryFrom<PeerId> for LocalSocketAddr {
    type Error = AddrError;

    fn try_from(peer_id: PeerId) -> Result<Self, Self::Error> {
        PartialNodeAddr::from_str(peer_id.as_inner())?.into()
    }
}

impl TryFrom<PeerId> for RemoteSocketAddr {
    type Error = AddrError;

    fn try_from(peer_id: PeerId) -> Result<Self, Self::Error> {
        RemoteSocketAddr::from_str(peer_id.as_inner())
    }
}

/// Hooks into service life cycle which may be implemented by service runtime
/// object
// TODO: Move into LNP/BP Services library
pub trait Hooks {
    fn on_ready(&mut self) -> Result<(), Error> {
        Ok(())
    }
}

pub struct Service<Runtime>
where
    Runtime: Hooks
        + esb::Handler<ServiceBus, Address = ServiceId, Request = Request>,
    esb::Error: From<Runtime::Error>,
{
    esb: esb::Controller<ServiceBus, Request, Runtime>,
    broker: bool,
}

impl<Runtime> Service<Runtime>
where
    Runtime: Hooks
        + esb::Handler<ServiceBus, Address = ServiceId, Request = Request>,
    esb::Error: From<Runtime::Error>,
{
    #[cfg(feature = "node")]
    pub fn run(
        config: Config,
        runtime: Runtime,
        broker: bool,
    ) -> Result<(), Error> {
        let service = Self::with(config, runtime, broker)?;
        service.run_loop()?;
        unreachable!()
    }

    fn with(
        config: Config,
        runtime: Runtime,
        broker: bool,
    ) -> Result<Self, esb::Error> {
        let router = if !broker {
            Some(ServiceId::router())
        } else {
            None
        };
        let esb = esb::Controller::with(
            map! {
                ServiceBus::Msg => esb::BusConfig::with_locator(
                        config.msg_endpoint.try_into()
                            .expect("Only ZMQ RPC is currently supported"),
                    router.clone()
                ),
                ServiceBus::Ctl => esb::BusConfig::with_locator(
                    config.ctl_endpoint.try_into()
                        .expect("Only ZMQ RPC is currently supported"),
                    router
                )
            },
            runtime,
            if broker {
                ZmqType::EsbService
            } else {
                ZmqType::EsbClient
            },
        )?;
        Ok(Self { esb, broker })
    }

    pub fn broker(
        config: Config,
        runtime: Runtime,
    ) -> Result<Self, esb::Error> {
        Self::with(config, runtime, true)
    }

    pub fn service(
        config: Config,
        runtime: Runtime,
    ) -> Result<Self, esb::Error> {
        Self::with(config, runtime, false)
    }

    pub fn is_broker(&self) -> bool {
        self.broker
    }

    pub fn add_loopback(
        &mut self,
        socket: zmq::Socket,
    ) -> Result<(), esb::Error> {
        self.esb.add_service_bus(
            ServiceBus::Bridge,
            esb::BusConfig {
                carrier: zmqsocket::Carrier::Socket(socket),
                router: None,
                queued: true,
            },
        )
    }

    #[cfg(feature = "node")]
    pub fn run_loop(mut self) -> Result<(), Error> {
        if !self.is_broker() {
            std::thread::sleep(core::time::Duration::from_secs(1));
            self.esb.send_to(
                ServiceBus::Ctl,
                ServiceId::Lnpd,
                Request::Hello,
            )?;
            self.esb.send_to(
                ServiceBus::Msg,
                ServiceId::Lnpd,
                Request::Hello,
            )?;
        }

        let identity = self.esb.handler().identity();
        info!("{} started", identity);

        self.esb.handler().on_ready()?;

        self.esb.run_or_panic(&identity.to_string());

        unreachable!()
    }
}

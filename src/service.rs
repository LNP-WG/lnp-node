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

use std::convert::TryInto;
use std::io;

use lnpbp::lnp::{zmqsocket, ChannelId, TempChannelId};
use lnpbp::strict_encoding::{
    self, strict_decode, strict_encode, StrictDecode, StrictEncode,
};
use lnpbp_services::esb;
#[cfg(feature = "node")]
use lnpbp_services::node::TryService;

use crate::rpc::{Request, ServiceBus};
use crate::Config;
#[cfg(feature = "node")]
use crate::Error;

/// Identifiers of daemons participating in LNP Node
#[derive(Clone, PartialEq, Eq, Hash, Debug, Display, From)]
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
    Connection(String),

    #[display("channel<{_0:#x}>")]
    #[from]
    #[from(TempChannelId)]
    Channel(ChannelId),

    #[display("external<{_0}>")]
    Foreign(String),
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
            ServiceId::Foreign(String::from_utf8_lossy(&vec).to_string())
        })
    }
}

impl StrictEncode for ServiceId {
    type Error = strict_encoding::Error;

    fn strict_encode<E: io::Write>(
        &self,
        mut e: E,
    ) -> Result<usize, Self::Error> {
        Ok(match self {
            ServiceId::Loopback => 0u8.strict_encode(e)?,
            ServiceId::Lnpd => 1u8.strict_encode(e)?,
            ServiceId::Gossip => 2u8.strict_encode(e)?,
            ServiceId::Routing => 3u8.strict_encode(e)?,
            ServiceId::Connection(peer_id) => {
                strict_encode_list!(e; 4u8, peer_id)
            }
            ServiceId::Channel(channel_id) => {
                strict_encode_list!(e; 5u8, channel_id)
            }
            ServiceId::Foreign(id) => strict_encode_list!(e; 6u8, id),
        })
    }
}

impl StrictDecode for ServiceId {
    type Error = strict_encoding::Error;

    fn strict_decode<D: io::Read>(mut d: D) -> Result<Self, Self::Error> {
        let ty = u8::strict_decode(&mut d)?;
        Ok(match ty {
            0 => ServiceId::Loopback,
            1 => ServiceId::Lnpd,
            2 => ServiceId::Gossip,
            3 => ServiceId::Routing,
            4 => ServiceId::Connection(StrictDecode::strict_decode(&mut d)?),
            5 => ServiceId::Channel(StrictDecode::strict_decode(&mut d)?),
            6 => ServiceId::Foreign(StrictDecode::strict_decode(&mut d)?),
            _ => Err(strict_encoding::Error::EnumValueNotKnown(
                s!("DaemonId"),
                ty,
            ))?,
        })
    }
}

pub struct Service<H>
where
    H: esb::Handler<ServiceBus, Address = ServiceId, Request = Request>,
    esb::Error: From<H::Error>,
{
    esb: esb::Controller<ServiceBus, Request, H>,
    broker: bool,
}

impl<H> Service<H>
where
    H: esb::Handler<ServiceBus, Address = ServiceId, Request = Request>,
    esb::Error: From<H::Error>,
{
    #[cfg(feature = "node")]
    pub fn run(config: Config, handler: H, broker: bool) -> Result<(), Error> {
        let service = Self::with(config, handler, broker)?;
        service.run_loop()?;
        unreachable!()
    }

    fn with(
        config: Config,
        handler: H,
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
            handler,
            if broker {
                zmqsocket::ApiType::EsbService
            } else {
                zmqsocket::ApiType::EsbClient
            },
        )?;
        Ok(Self { esb, broker })
    }

    pub fn broker(config: Config, handler: H) -> Result<Self, esb::Error> {
        Self::with(config, handler, true)
    }

    pub fn service(config: Config, handler: H) -> Result<Self, esb::Error> {
        Self::with(config, handler, false)
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
        self.esb.run_or_panic(&identity.to_string());
        unreachable!()
    }
}

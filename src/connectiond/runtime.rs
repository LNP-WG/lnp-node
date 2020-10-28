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

use amplify::Bipolar;
use core::convert::TryInto;
use std::collections::HashMap;
use std::thread::spawn;

use lnpbp::bitcoin::secp256k1::rand::{self, Rng};
use lnpbp::lnp::application::{message, Messages};
use lnpbp::lnp::presentation::Encode;
use lnpbp::lnp::transport::zmqsocket::{self, ZMQ_CONTEXT};
use lnpbp::lnp::{session, transport};
use lnpbp::lnp::{PeerConnection, PeerSender, SendMessage, TypedEnum};
use lnpbp_services::esb;
use lnpbp_services::node::TryService;
use lnpbp_services::{peer, rpc};

use crate::rpc::{Endpoints, Request};
use crate::{Config, DaemonId, Error};

pub struct MessageFilter {}

pub struct ServiceId {}

pub fn run(
    config: Config,
    connection: PeerConnection,
    id: String,
) -> Result<(), Error> {
    debug!("Splitting connection into receiver and sender parts");
    let (receiver, sender) = connection.split();

    debug!("Opening bridge between runtime and peer listener threads");
    let tx = ZMQ_CONTEXT.socket(zmq::PAIR)?;
    let rx = ZMQ_CONTEXT.socket(zmq::PAIR)?;
    tx.connect("inproc://bridge")?;
    rx.bind("inproc://bridge")?;

    debug!("Starting listening thread for messages from the remote peer");
    let processor = Processor {
        bridge: session::Raw::from_pair_socket(
            zmqsocket::ApiType::EsbService,
            tx,
        ),
    };
    let listener = peer::Listener::with(receiver, processor);
    spawn(move || listener.run_or_panic("connectiond-listener"));
    //.join()
    //.expect("Error joining receiver thread");

    debug!("Staring RPC service runtime");
    let runtime = Runtime {
        routing: none!(),
        sender,
        awaited_pong: None,
    };
    let rpc = esb::Controller::init(
        DaemonId::Connection(id),
        map! {
            Endpoints::Msg => rpc::EndpointCarrier::Address(
                config.msg_endpoint.try_into()
                    .expect("Only ZMQ RPC is currently supported")
            ),
            Endpoints::Ctl => rpc::EndpointCarrier::Address(
                config.ctl_endpoint.try_into()
                    .expect("Only ZMQ RPC is currently supported")
            ),
            Endpoints::Bridge => rpc::EndpointCarrier::Socket(rx)
        },
        runtime,
        zmqsocket::ApiType::EsbService,
    )?;

    info!("connectiond started");
    rpc.run_or_panic("connectiond-runtime");
    unreachable!()
}

pub struct Runtime {
    #[allow(dead_code)]
    routing: HashMap<ServiceId, MessageFilter>,
    sender: PeerSender,
    awaited_pong: Option<u16>,
}

pub struct Processor {
    bridge:
        session::Raw<session::NoEncryption, transport::zmqsocket::Connection>,
}

impl Processor {
    fn send_over_bridge(&mut self, req: Request) -> Result<(), Error> {
        debug!("Forwarding LNPWP message over BRIDGE interface to the runtime");
        self.bridge.send_addr_message(b"", &req.encode()?)?;
        Ok(())
    }
}

impl peer::Handler for Processor {
    type Error = crate::Error;

    fn handle(&mut self, message: Messages) -> Result<(), Self::Error> {
        // Forwarding all received messages to the runtime
        debug!("LNPWP message from peer: {}", message);
        trace!("LNPWP message details: {:?}", message);
        self.send_over_bridge(Request::LnpwpMessage(message))
    }

    fn handle_err(&mut self, err: Self::Error) -> Result<(), Self::Error> {
        debug!("Underlying peer interface requested to handle {:?}", err);
        match err {
            Error::Transport(transport::Error::TimedOut) => {
                trace!("Time to ping the remote peer");
                // This means socket reading timeout and the fact that we need
                // to send a ping message
                self.send_over_bridge(Request::PingPeer)
            }
            // for all other error types, indicating internal errors, we
            // propagate error to the upper level
            _ => {
                error!("Unrecoverable peer error {:?}, halting", err);
                Err(err)
            }
        }
    }
}

impl esb::Handler<Endpoints> for Runtime {
    type Request = Request;
    type Address = DaemonId;
    type Error = Error;

    fn handle(
        &mut self,
        senders: &mut esb::Senders<Endpoints>,
        endpoint: Endpoints,
        source: DaemonId,
        request: Request,
    ) -> Result<(), Self::Error> {
        match endpoint {
            Endpoints::Msg => self.handle_rpc_msg(senders, source, request),
            Endpoints::Ctl => self.handle_rpc_ctl(senders, source, request),
            Endpoints::Bridge => self.handle_bridge(senders, source, request),
        }
    }
}

impl Runtime {
    fn handle_rpc_msg(
        &mut self,
        _senders: &mut esb::Senders<Endpoints>,
        source: DaemonId,
        request: Request,
    ) -> Result<(), Error> {
        debug!("MSG RPC request from {}: {}", source, request);
        match request {
            Request::LnpwpMessage(message) => {
                // 1. Check permissions
                // 2. Forward to the remote peer
                debug!("Forwarding LN peer message to the remote peer");
                trace!("Message details: {:?}", message);
                self.sender.send_message(message)?;
            }
            _ => {
                error!(
                    "MSG RPC can be only used for forwarding LNPWP messages"
                );
                return Err(Error::NotSupported(
                    Endpoints::Msg,
                    request.get_type(),
                ));
            }
        }
        Ok(())
    }

    fn handle_rpc_ctl(
        &mut self,
        _senders: &mut esb::Senders<Endpoints>,
        source: DaemonId,
        request: Request,
    ) -> Result<(), Error> {
        debug!("CTL RPC request from {}: {}", source, request);
        match request {
            /* TODO: Move to connection initialization
            Request::InitConnection => {
                debug!("Initializing connection with the remote peer");
                self.sender.send_message(Messages::Init(message::Init {
                    global_features: none!(),
                    local_features: none!(),
                    assets: none!(),
                    unknown_tlvs: none!(),
                }))?;
            }
             */
            Request::PingPeer => {
                debug!("Requested to ping remote peer");
                self.ping()?;
            }
            _ => {
                error!("Request is not supported by the CTL interface");
                return Err(Error::NotSupported(
                    Endpoints::Ctl,
                    request.get_type(),
                ));
            }
        }
        Ok(())
    }

    fn handle_bridge(
        &mut self,
        senders: &mut esb::Senders<Endpoints>,
        _source: DaemonId,
        request: Request,
    ) -> Result<(), Error> {
        debug!("BRIDGE RPC request: {}", request);
        match request {
            Request::PingPeer => {
                self.ping()?;
            }

            Request::LnpwpMessage(Messages::Ping(message::Ping {
                pong_size,
                ..
            })) => {
                self.pong(pong_size)?;
            }

            Request::LnpwpMessage(Messages::Pong(noise)) => {
                match self.awaited_pong {
                    None => error!("Unexpected pong from the remote peer"),
                    Some(len) if len as usize != noise.len() => warn!(
                        "Pong data size does not match requested with ping"
                    ),
                    _ => trace!("Got pong reply, exiting pong await mode"),
                }
                self.awaited_pong = None;
            }

            Request::LnpwpMessage(Messages::OpenChannel(_)) => {
                senders.send_to(Endpoints::Msg, DaemonId::Lnpd, request)?;
            }

            Request::LnpwpMessage(message) => {
                // 1. Check permissions
                // 2. Forward to the corresponding daemon
                debug!("Got peer LNPWP message {}", message);
            }

            _ => {
                error!("Request is not supported by the BRIDGE interface");
                return Err(Error::NotSupported(
                    Endpoints::Bridge,
                    request.get_type(),
                ))?;
            }
        }
        Ok(())
    }

    fn ping(&mut self) -> Result<(), Error> {
        trace!("Sending ping to the remote peer");
        if self.awaited_pong.is_some() {
            return Err(Error::NotResponding);
        }
        let mut rng = rand::thread_rng();
        let len: u16 = rng.gen_range(4, 32);
        let mut noise = vec![0u8; len as usize];
        for i in 0..noise.len() {
            noise[i] = rng.gen();
        }
        let pong_size = rng.gen_range(4, 32);
        self.sender.send_message(Messages::Ping(message::Ping {
            ignored: noise,
            pong_size,
        }))?;
        self.awaited_pong = Some(pong_size);
        Ok(())
    }

    fn pong(&mut self, pong_size: u16) -> Result<(), Error> {
        trace!("Replying with pong to the remote peer");
        let mut noise = vec![0u8; pong_size as usize];
        let mut rng = rand::thread_rng();
        for i in 0..noise.len() {
            noise[i] = rng.gen();
        }
        self.sender.send_message(Messages::Pong(noise))?;
        Ok(())
    }
}

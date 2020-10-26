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

use lnpbp::lnp::application::{message, Messages};
use lnpbp::lnp::presentation::Encode;
use lnpbp::lnp::ZMQ_CONTEXT;
use lnpbp::lnp::{
    PeerConnection, PeerReceiver, PeerSender, RecvMessage, SendMessage,
    TypedEnum,
};
use lnpbp::strict_encoding::StrictEncode;
use lnpbp_services::node::TryService;
use lnpbp_services::rpc::{Failure, Handler};
use lnpbp_services::server::{EndpointCarrier, RpcZmqServer};
use lnpbp_services::{peer, rpc};

use crate::rpc::{Endpoints, Reply, Request, Rpc};
use crate::{Config, Error};

pub struct MessageFilter {}

pub struct ServiceId {}

pub fn run(connection: PeerConnection, config: Config) -> Result<(), Error> {
    debug!("Splitting connection into receiver and sender parts");
    let (receiver, sender) = connection.split();

    debug!("Opening bridge between runtime and peer listener threads");
    let tx = ZMQ_CONTEXT.socket(zmq::PAIR)?;
    let rx = ZMQ_CONTEXT.socket(zmq::PAIR)?;
    tx.connect("inproc://bridge")?;
    rx.bind("inproc://bridge")?;

    debug!("Starting listening thread for messages from the remote peer");
    let processor = Processor { bridge: tx };
    let listener = peer::Listener::with(receiver, processor);
    spawn(move || listener.run_or_panic("connectiond-listener"));
    //.join()
    //.expect("Error joining receiver thread");

    debug!("Staring RPC service runtime");
    let runtime = Runtime {
        routing: none!(),
        sender,
    };
    let rpc = RpcZmqServer::init(
        map! {
            Endpoints::Msg => EndpointCarrier::Address(
                config.msg_endpoint.try_into()
                    .expect("Only ZMQ RPC is currently supported")
            ),
            Endpoints::Ctl => EndpointCarrier::Address(
                config.ctl_endpoint.try_into()
                    .expect("Only ZMQ RPC is currently supported")
            ),
            Endpoints::Bridge => EndpointCarrier::Socket(rx)
        },
        runtime,
    )?;
    rpc.run_or_panic("connectiond-runtime");
    unreachable!()
}

pub struct Runtime {
    pub(super) routing: HashMap<ServiceId, MessageFilter>,
    pub(super) sender: PeerSender,
}

pub struct Processor {
    pub(super) bridge: zmq::Socket,
}

impl peer::Handler for Processor {
    type Error = crate::Error;

    fn handle(&mut self, message: Messages) -> Result<(), Self::Error> {
        // Forwarding all received messages to the runtime
        let req = Request::LnpwpMessage(message);
        self.bridge.send(req.encode()?, 0)?;
        Ok(())
    }

    fn handle_err(&mut self, err: Self::Error) -> Result<(), Self::Error> {
        match err {
            // for all other error types, indicating internal errors, we
            // propagate error to the upper level
            _ => Err(err),
        }
    }
}

impl Handler<Endpoints> for Runtime {
    type Api = Rpc;
    type Error = Error;

    fn handle(
        &mut self,
        endpoint: Endpoints,
        request: Request,
    ) -> Result<Reply, Self::Error> {
        match endpoint {
            Endpoints::Msg => self.handle_rpc_msg(request),
            Endpoints::Ctl => self.handle_rpc_ctl(request),
            Endpoints::Bridge => self.handle_bridge(request),
        }
    }
}

impl Runtime {
    fn handle_rpc_msg(&mut self, request: Request) -> Result<Reply, Error> {
        match request {
            Request::LnpwpMessage(message) => {
                // 1. Check permissions
                // 2. Forward to the remote peer
                self.sender.send_message(message)?;
                Ok(Reply::Success)
            }
            _ => Err(Error::NotSupported(Endpoints::Msg, request.get_type())),
        }
    }

    fn handle_rpc_ctl(&mut self, request: Request) -> Result<Reply, Error> {
        match request {
            Request::InitConnection => {
                self.sender.send_message(Messages::Init(message::Init {
                    global_features: none!(),
                    local_features: none!(),
                    assets: none!(),
                    unknown_tlvs: none!(),
                }))?;
                Ok(Reply::Success)
            }
            Request::PingPeer => {
                self.sender.send_message(Messages::Ping)?;
                Ok(Reply::Success)
            }
            _ => Err(Error::NotSupported(Endpoints::Ctl, request.get_type())),
        }
    }

    fn handle_bridge(&mut self, request: Request) -> Result<Reply, Error> {
        match request {
            Request::LnpwpMessage(Messages::Ping) => {
                self.sender.send_message(Messages::Ping)?;
                Ok(Reply::Success)
            }
            Request::LnpwpMessage(message) => {
                // 1. Check permissions
                // 2. Forward to the corresponding daemon
                Ok(Reply::Success)
            }
            _ => {
                Err(Error::NotSupported(Endpoints::Bridge, request.get_type()))
            }
        }
    }
}

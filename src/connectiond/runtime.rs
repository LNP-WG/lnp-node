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

use lnpbp::lnp::ZMQ_CONTEXT;
use lnpbp::lnp::{PeerConnection, PeerReceiver, PeerSender};
use lnpbp_services::node::TryService;
use lnpbp_services::server::{EndpointCarrier, RpcZmqServer};

use super::Config;
use crate::rpc::Endpoints;
use crate::Error;

pub struct MessageFilter {}

pub struct ServiceId {}

pub fn run(connection: PeerConnection, config: Config) -> Result<(), Error> {
    debug!("Splitting connection into receiver and sender parts");
    let (receiver, sender) = connection.split();

    debug!("Opening bridge between runtime sender/receiver threads");
    let tx = ZMQ_CONTEXT.socket(zmq::PAIR)?;
    let rx = ZMQ_CONTEXT.socket(zmq::PAIR)?;
    tx.connect("inproc://bridge")?;
    rx.bind("inproc://bridge")?;

    debug!("Starting thread listening for messages from the remote peer");
    let thread = Thread {
        receiver,
        bridge: tx,
    };
    spawn(move || thread.run_or_panic("connectiond-thread"))
        .join()
        .expect("Error joining receiver thread");

    debug!("Staring service sending messages to the remote peer");
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
                    .expect("Only ZMQ RPC is currently supported"))
            ,
            Endpoints::Bridge => EndpointCarrier::Socket(rx)
        },
        runtime,
    )?;
    rpc.run_or_panic("connectiond-runtime");
    unreachable!()
}

pub struct Runtime {
    pub(self) routing: HashMap<ServiceId, MessageFilter>,
    pub(self) sender: PeerSender,
}

pub struct Thread {
    pub(self) receiver: PeerReceiver,
    pub(self) bridge: zmq::Socket,
}

impl TryService for Thread {
    type ErrorType = Error;

    fn try_run_loop(self) -> Result<(), Self::ErrorType> {
        debug!("Entering event loop of the sender service");
        loop {
            debug!(
                "Awaiting for incoming MSG, CTL and BRIDGE interface messages"
            );
            // let msg = self.receiver.recv_message()?;
            // TODO: Convert message into RPC format
            // self.bridge.send(&msg.as_bytes(), 0)?;
        }
    }
}

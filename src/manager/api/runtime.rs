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

use tokio::{
    sync::Mutex,
    task::JoinHandle
};
use std::sync::Arc;
use log::*;
use crate::{
    error::*,
    INPUT_PARSER_SOCKET,
    PARSER_PUB_SOCKET
};
use super::*;
use responder::*;
use publisher::*;

pub fn run(config: Config, context: &mut zmq::Context)
           -> Result<Vec<JoinHandle<!>>, BootstrapError>
{
    let req_socket_addr = config.req_socket.clone();
    let pub_socket_addr = config.pub_socket.clone();

    // Opening IPC socket to parser thread
    let parser = context.socket(zmq::REQ)
        .map_err(|e| BootstrapError::IPCSocketError(e, IPCSocket::Input2Parser, None))?;
    parser.bind(INPUT_PARSER_SOCKET)
        .map_err(|e| BootstrapError::IPCSocketError(e, IPCSocket::Input2Parser,
                                                    Some(INPUT_PARSER_SOCKET.into())))?;
    debug!("IPC ZMQ from Input to Parser threads is opened on Input runtime side");

    // Opening parser Sub socket
    let subscriber = context.socket(zmq::SUB)
        .map_err(|e| BootstrapError::InputSocketError(e, APISocket::PubSub, None))?;
    subscriber.connect(PARSER_PUB_SOCKET)
        .map_err(|e| BootstrapError::InputSocketError(e, APISocket::PubSub,
                                                  Some(PARSER_PUB_SOCKET.into())))?;
    subscriber.set_subscribe("".as_bytes())
        .map_err(|e| BootstrapError::InputSocketError(e, APISocket::PubSub,
                                                      Some(PARSER_PUB_SOCKET.into())))?;
    debug!("Input thread subscribed to Parser service PUB notifications");

    // Opening input API Req/Rep socket
    let responder = context.socket(zmq::REP)
        .map_err(|e| BootstrapError::InputSocketError(e, APISocket::ReqRep, None))?;
    responder.bind(req_socket_addr.as_str())
        .map_err(|e| BootstrapError::InputSocketError(e, APISocket::ReqRep, Some(req_socket_addr.clone())))?;
    debug!("Input Req/Rep ZMQ API is opened on {}", req_socket_addr);

    // Opening input API Pub/Sub socket
    let publisher = context.socket(zmq::PUB)
        .map_err(|e| BootstrapError::InputSocketError(e, APISocket::PubSub, None))?;
    publisher.bind(pub_socket_addr.as_str())
        .map_err(|e| BootstrapError::InputSocketError(e, APISocket::PubSub, Some(pub_socket_addr.clone())))?;
    debug!("Input Pub/Sub ZMQ API is opened on {}", pub_socket_addr);

    // Thread synchronization flag
    let busy = Arc::new(Mutex::new(false));

    let responder_service = ResponderService::init(config.clone().into(), responder, parser, &busy);
    let publisher_service = PublisherService::init(config.clone().into(), publisher, subscriber, &busy);

    Ok(vec![
        tokio::spawn(async move {
            info!("Api responder service is listening on {}", req_socket_addr);
            responder_service.run_loop().await
        }),
        tokio::spawn(async move {
            info!("Client notifier service is listening on {}", pub_socket_addr);
            publisher_service.run_loop().await
        }),
    ])
}

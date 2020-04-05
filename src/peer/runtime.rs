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


use tiny_http;
use tokio::{
    task::JoinHandle,
    net::TcpListener,
};
use prometheus::Encoder;

use crate::{
    error::*,
    Service
};
use super::*;

pub fn run(config: Config, context: &mut zmq::Context)
           -> Result<Vec<JoinHandle<!>>, BootstrapError>
{
    let socket_addr = config.socket.clone();
    let socket = TcpListener::bind(socket_addr.clone()).await?;

    let peer_service = PeerService::init(
        config,
        socket,
    );

    Ok(vec![
        tokio::spawn(async move {
            info!("Peer service is listening on {}", socket_addr);
            peer_service.run_loop().await
        }),
    ])
}

struct PeerService {
    config: Config,
    listener: TcpListener,
    nodes: Vec<Node>,
    peers: HashMap<Node, Peer>,
}

#[async_trait]
impl Service for PeerService {
    async fn run_loop(mut self) -> ! {
        loop {
            match self.run().await {
                Ok(_) => debug!("Monitoring client request processing completed"),
                Err(err) => {
                    error!("Error processing monitoring client request: {}", err)
                },
            }
        }
    }
}

impl PeerService {
    pub fn init(config: Config,
                listener: TcpListener) -> Self {
        Self {
            config,
            listener,
            peers: vec![]
        }
    }

    async fn run(&mut self) -> Result<(), Error> {
        let (socket, stream) = self.listener.accept().await?;

        // TODO: Do we need to join handle here? Seems like no: the PeerService
        //       runs endlessly until being terminated as a part of the daemon,
        //       so all per-peer connections will persist w/o the need to
        //       join them
        tokio::spawn(async move {
            let peer = ConnectedPeer::from_incoming(socket, stream)?;
            self.peers.push(peer);

        })?;

        Ok(())
    }
}

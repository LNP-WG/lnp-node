// Lightning network protocol (LNP) daemon suite
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


use std::net::SocketAddr;
use std::sync::Arc;
use std::convert::TryFrom;
use tokio::{
    sync::Mutex,
    task::JoinHandle,
    net::{TcpListener, TcpStream}
};

use crate::TryService;
use crate::wired::BootstrapError;
use crate::wired::peer::{self, PeerService};
use super::*;

pub struct WireService {
    config: Config,
    peer_config: peer::Config,
    context: zmq::Context,
    listener: TcpListener,
    sockets: Arc<Mutex<Vec<Arc<TcpStream>>>>,
    threads: Arc<Mutex<Vec<JoinHandle<!>>>>,
}

#[async_trait]
impl TryService for WireService {
    type ErrorType = Error;

    async fn try_run_loop(mut self) -> Result<!, Error> {
        loop {
            match self.run().await {
                Ok(_) => debug!("New LN peer was successfully connected"),
                Err(err) => {
                    error!("Error connecting new LN peer: {}", err)
                },
            }
        }
    }
}

impl WireService {
    pub async fn init(
        config: Config,
        peer_config: peer::Config,
        context: zmq::Context,
        sockets: Arc<Mutex<Vec<Arc<TcpStream>>>>,
        threads: Arc<Mutex<Vec<JoinHandle<!>>>>
    ) -> Result<Self, BootstrapError> {
        debug!("Opening LN P2P socket at {}", config.lnp2p_addr);

        /*
        if config.lnp2p_addr.is_tor() {
            Err(BootstrapError::TorNotYetSupported)?;
        }
        */

        let addr = SocketAddr::try_from(config.lnp2p_addr)
            .expect("Non-Tor address failed to convert into an IP address");
        let listener = TcpListener::bind(addr).await?;

        info!("Listening for incoming LN P2P connections at {}", config.lnp2p_addr);

        Ok(Self {
            config,
            peer_config,
            context,
            listener,
            sockets,
            threads
        })
    }

    async fn run(&mut self) -> Result<(), Error> {
        let (stream, addr) = self.listener.accept().await?;
        info!("New LN peer connected: {}", addr);

        debug!("Instantiating new peer service for {}", addr);
        let stream = Arc::new(stream);
        self.sockets.lock().await.push(stream.clone());

        let service = PeerService::init(self.peer_config.clone(), self.context.clone(), stream)?;

        let handle = tokio::spawn(async move {
            trace!("Running peer service for {}", addr);
            service.run_or_panic(&format!("Peer service for {}", addr)).await
        });
        self.threads.lock().await.push(handle);

        Ok(())
    }
}

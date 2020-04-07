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


use std::sync::Arc;
use tokio::{
    sync::Mutex,
    net::TcpStream,
    task::JoinHandle,
};
use lnpbp::internet::InetSocketAddr;

use crate::{Service, TryService, BootstrapError};
use super::*;


#[derive(Copy, Clone, PartialEq, Eq, Debug, Display)]
#[display_from(Debug)]
pub enum ConnDirection {
    In,
    Out,
}


#[derive(Debug, Display)]
#[display_from(Debug)]
pub struct PeerConnection {
    pub stream: Arc<TcpStream>,
    pub address: InetSocketAddr,
    pub direction: ConnDirection,
    pub thread: JoinHandle<!>,
}

pub type PeerConnectionList = Arc<Mutex<Vec<PeerConnection>>>;


pub struct Runtime {
    config: Config,
    context: zmq::Context,
    peer_connections: PeerConnectionList,
    wire_service: WireService,
    bus_service: BusService,
}

impl Runtime {
    pub async fn init(config: Config) -> Result<Self, BootstrapError> {
        let context = zmq::Context::new();

        let peer_connections = Arc::new(Mutex::new(vec![]));

        let wire_service = WireService::init(
            config.clone().into(),
            config.clone().into(),
            context.clone(),
            peer_connections.clone(),
        ).await?;
        let bus_service = BusService::init(
            config.clone().into(),
            context.clone(),
            peer_connections.clone()
        )?;

        Ok(Self {
            config,
            context,
            peer_connections,
            wire_service,
            bus_service
        })
    }
}

#[async_trait]
impl TryService for Runtime {
    type ErrorType = tokio::task::JoinError;

    async fn try_run_loop(self) -> Result<!, Self::ErrorType> {
        let wire_addr = self.config.lnp2p_addr.clone();
        let bus_addr = self.config.msgbus_peer_api_addr.clone();

        let wire_service = self.wire_service;
        let bus_service = self.bus_service;

        try_join!(
            tokio::spawn(async move {
                info!("LN P2P wire service is running on {}", wire_addr);
                wire_service.run_or_panic("LN P2P service").await
            }),
            tokio::spawn(async move {
                info!("Message bus service is listening on {}", bus_addr);
                bus_service.run_loop().await
            })
        )?;

        loop { }
    }
}

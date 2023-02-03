use lnp::p2p;
use lnp::p2p::bifrost::SwapId;
use lnp_rpc::{RpcMsg, ServiceId, SwapInfo};
use microservices::esb::{self, EndpointList, Handler};

use crate::bus::{BusMsg, CtlMsg, ServiceBus};
use crate::{Config, Endpoints, Error, Responder, Service};

pub fn run(swap_id: SwapId, config: Config) -> Result<(), Error> {
    debug!("Opening bridge between electrum watcher and main service threads");
    let runtime = Runtime::with(swap_id, &config);
    Service::run(config, runtime, false)
}

pub struct Runtime {
    enquirer: Option<esb::ClientId>,
    id: SwapId,
}

impl Responder for Runtime {
    #[inline]
    fn enquirer(&self) -> Option<esb::ClientId> { self.enquirer }
}

impl Handler<ServiceBus> for Runtime {
    type Request = BusMsg;
    type Error = Error;

    fn identity(&self) -> <ServiceBus as esb::BusId>::Address { ServiceId::Swapd(self.id.clone()) }

    fn handle(
        &mut self,
        endpoints: &mut esb::EndpointList<ServiceBus>,
        bus_id: ServiceBus,
        source: ServiceId,
        request: Self::Request,
    ) -> Result<(), Self::Error> {
        match (bus_id, request, source) {
            (ServiceBus::Msg, BusMsg::Bifrost(msg), ServiceId::PeerBifrost(_nodeid)) => {
                self.handle_peerswap_msg(endpoints, msg)
            }
            (ServiceBus::Msg, BusMsg::Bifrost(_), service) => unreachable!(
                "swapd received bifrost p2p message not from a peerd but from {}",
                service
            ),
            (ServiceBus::Rpc, BusMsg::Rpc(msg), ServiceId::Client(client_id)) => {
                self.handle_rpc(endpoints, client_id, msg)
            }
            (ServiceBus::Rpc, BusMsg::Rpc(_), service) => {
                unreachable!("swapd received RPC message not from a client but from {}", service)
            }
            (ServiceBus::Ctl, BusMsg::Ctl(msg), source) => self.handle_ctl(endpoints, source, msg),
            (bus, msg, _) => Err(Error::wrong_esb_msg(bus, &msg)),
        }
    }

    fn handle_err(
        &mut self,
        endpoints: &mut esb::EndpointList<ServiceBus>,
        error: esb::Error<<ServiceBus as esb::BusId>::Address>,
    ) -> Result<(), Self::Error> {
        todo!()
    }
}

impl Runtime {
    pub fn with(swap_id: SwapId, config: &Config) -> Self { todo!() }

    fn handle_ctl(
        &mut self,
        endpoints: &mut Endpoints,
        source: ServiceId,
        msg: CtlMsg,
    ) -> Result<(), Error> {
        todo!()
    }

    fn handle_rpc(
        &self,
        endpoints: &mut Endpoints,
        client_id: u64,
        msg: RpcMsg,
    ) -> Result<(), <Self as Handler<ServiceBus>>::Error> {
        match msg {
            RpcMsg::GetInfo => {
                let swap_info = RpcMsg::SwapInfo(SwapInfo {
                    id: self.id.clone(),
                });
                self.send_rpc(endpoints, client_id, swap_info)?;
            },
            wrong_request => {
                error!("Request is not supported by the RPC interface");
                return Err(Error::wrong_esb_msg(ServiceBus::Rpc, &wrong_request));
            }
        }
        Ok(())
    }
    fn handle_peerswap_msg(
        &self,
        endpoints: &mut EndpointList<ServiceBus>,
        msg: p2p::bifrost::Messages,
    ) -> Result<(), <Self as Handler<ServiceBus>>::Error> {
        match msg {
            p2p::bifrost::Messages::SwapInRequest(req) => {
                todo!()
            }
            _ => todo!(),
        }
    }
}

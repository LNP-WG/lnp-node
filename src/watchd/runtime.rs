// LNP Node: node running lightning network protocol and generalized lightning
// channels.
// Written in 2020-2022 by
//     Dr. Maxim Orlovsky <orlovsky@lnp-bp.org>
//
// To the extent possible under law, the author(s) have dedicated all
// copyright and related and neighboring rights to this software to
// the public domain worldwide. This software is distributed without
// any warranty.
//
// You should have received a copy of the MIT License along with this software.
// If not, see <https://opensource.org/licenses/MIT>.

use std::collections::HashMap;
use std::sync::mpsc;
use std::thread::spawn;

use bitcoin::Txid;
use internet2::zeromq;
use lnp::p2p::bolt::Messages as LnMsg;
use microservices::node::TryService;
use microservices::{esb, ZMQ_CONTEXT};

use crate::bus::{BusMsg, CtlMsg, ServiceBus};
use crate::rpc::ServiceId;
use crate::watchd::{ElectrumUpdate, ElectrumWorker};
use crate::{BridgeHandler, Config, Endpoints, Error, Service};

pub fn run(config: Config) -> Result<(), Error> {
    debug!("Opening bridge between electrum watcher and main service threads");
    let tx = ZMQ_CONTEXT.socket(zmq::PAIR)?;
    let rx = ZMQ_CONTEXT.socket(zmq::PAIR)?;
    tx.connect("inproc://electrum-bridge")?;
    rx.bind("inproc://electrum-bridge")?;

    let (sender, receiver) = mpsc::channel::<ElectrumUpdate>();
    let electrum_worker = ElectrumWorker::with(sender, &config.electrum_url, 15)?;

    debug!("Starting electrum watcher thread");
    let watcher_runtime = WatcherRuntime::with(receiver, tx)?;
    spawn(move || watcher_runtime.run_or_panic("electrum watcher"));

    let runtime = Runtime { electrum_worker, track_list: empty!() };
    let mut service = Service::service(config, runtime)?;
    service.add_loopback(rx)?;
    service.run_loop()?;
    unreachable!()
}

struct WatcherRuntime {
    identity: ServiceId,
    bridge: esb::Controller<ServiceBus, BusMsg, BridgeHandler>,
    receiver: mpsc::Receiver<ElectrumUpdate>,
}

impl WatcherRuntime {
    pub fn with(
        receiver: mpsc::Receiver<ElectrumUpdate>,
        tx: zmq::Socket,
    ) -> Result<WatcherRuntime, Error> {
        let bridge = esb::Controller::with(
            map! {
                ServiceBus::Bridge => esb::BusConfig {
                    api_type: zeromq::ZmqSocketType::Rep,
                    carrier: zeromq::Carrier::Socket(tx),
                    router: None,
                    queued: true,
                    topic: None,
                }
            },
            BridgeHandler,
        )?;

        Ok(Self { identity: ServiceId::Loopback, receiver, bridge })
    }

    pub(self) fn send_over_bridge(&mut self, req: BusMsg) -> Result<(), Error> {
        debug!("Forwarding electrum update message over BRIDGE interface to the runtime");
        self.bridge.send_to(ServiceBus::Bridge, ServiceId::Watch, req)?;
        Ok(())
    }

    fn run(&mut self) -> Result<(), mpsc::RecvError> {
        trace!("Awaiting for electrum update...");
        let msg = self.receiver.recv()?;
        debug!("Processing message {}", msg);
        trace!("Message details: {:?}", msg);
        // TODO: Forward all electrum notifications over the bridge
        // self.send_over_bridge(msg.into()).expect("watcher bridge is halted");
        match msg {
            ElectrumUpdate::TxConfirmations(transactions, _) => {
                for transaction in transactions {
                    self.send_over_bridge(BusMsg::Ctl(CtlMsg::TxFound(transaction)))
                        .expect("unable forward electrum notifications over the bridge");
                }
            }
            ElectrumUpdate::TxBatch(..)
            | ElectrumUpdate::Connecting
            | ElectrumUpdate::Connected
            | ElectrumUpdate::Complete
            | ElectrumUpdate::FeeEstimate(..)
            | ElectrumUpdate::LastBlock(_)
            | ElectrumUpdate::LastBlockUpdate(_)
            | ElectrumUpdate::ChannelDisconnected
            | ElectrumUpdate::Error(_) => { /* nothing to do here */ }
        }

        Ok(())
    }
}

impl TryService for WatcherRuntime {
    type ErrorType = mpsc::RecvError;

    fn try_run_loop(mut self) -> Result<(), Self::ErrorType> {
        trace!("Entering event loop of the watcher service");
        loop {
            self.run()?;
            trace!("Electrum update processing complete");
        }
    }
}

pub struct Runtime {
    electrum_worker: ElectrumWorker,
    track_list: HashMap<Txid, (u32, ServiceId)>,
}

impl esb::Handler<ServiceBus> for Runtime {
    type Request = BusMsg;
    type Error = Error;

    fn identity(&self) -> ServiceId { ServiceId::Watch }

    fn handle(
        &mut self,
        endpoints: &mut Endpoints,
        bus: ServiceBus,
        source: ServiceId,
        message: BusMsg,
    ) -> Result<(), Self::Error> {
        match (bus, message, source) {
            (ServiceBus::Bridge, BusMsg::Ctl(msg), ServiceId::Loopback) => {
                self.handle_bridge(endpoints, msg)
            }
            (ServiceBus::Msg, BusMsg::Bolt(msg), source) => self.handle_p2p(endpoints, source, msg),
            (ServiceBus::Ctl, BusMsg::Ctl(msg), source) => self.handle_ctl(endpoints, source, msg),
            (bus, msg, _) => Err(Error::wrong_esb_msg(bus, &msg)),
        }
    }

    fn handle_err(
        &mut self,
        _: &mut Endpoints,
        _: esb::Error<ServiceId>,
    ) -> Result<(), Self::Error> {
        // We do nothing and do not propagate error; it's already being reported
        // with `error!` macro by the controller. If we propagate error here
        // this will make whole daemon panic
        Ok(())
    }
}

impl Runtime {
    fn handle_bridge(&mut self, endpoints: &mut Endpoints, request: CtlMsg) -> Result<(), Error> {
        debug!("BRIDGE RPC request: {}", request);

        match request {
            CtlMsg::TxFound(tx_status) => {
                if let Some((required_height, service_id)) = self.track_list.get(&tx_status.txid) {
                    if *required_height <= tx_status.confirmations {
                        let service_id = service_id.clone();
                        self.untrack(tx_status.txid);
                        match self.electrum_worker.untrack_transaction(tx_status.txid) {
                            Ok(_) => debug!("Untracking tx {}", tx_status.txid),
                            _ => error!("Unable untrack transaction in electrum worker"),
                        }
                        endpoints.send_to(
                            ServiceBus::Ctl,
                            ServiceId::Watch,
                            service_id,
                            BusMsg::Ctl(CtlMsg::TxFound(tx_status)),
                        )?;
                    }
                }
                Ok(())
            }

            wrong_msg => {
                error!("Request is not supported by the BRIDGE interface");
                Err(Error::wrong_esb_msg(ServiceBus::Bridge, &wrong_msg))
            }
        }
    }

    fn handle_p2p(
        &mut self,
        _endpoints: &mut Endpoints,
        _source: ServiceId,
        message: LnMsg,
    ) -> Result<(), Error> {
        #[allow(clippy::match_single_binding)]
        match message {
            _ => {
                // TODO: Process message
            }
        }
        Ok(())
    }

    fn handle_ctl(
        &mut self,
        _: &mut Endpoints,
        source: ServiceId,
        message: CtlMsg,
    ) -> Result<(), Error> {
        match message {
            CtlMsg::Track { txid, depth } => {
                debug!("Tracking status for tx {txid}");
                self.track_list.insert(txid, (depth, source));
                match self.electrum_worker.track_transaction(txid) {
                    Ok(_) => debug!("Tracking status for tx {txid}"),
                    _ => error!("Unable track transaction in electrum worker"),
                }
            }
            CtlMsg::Untrack(txid) => {
                self.untrack(txid);
                match self.electrum_worker.untrack_transaction(txid) {
                    Ok(_) => debug!("Untracking tx {txid}"),
                    _ => error!("Unable untrack transaction in electrum worker"),
                }
            }

            wrong_msg => {
                error!("Request {} is not supported by the CTL interface", wrong_msg);
                return Err(Error::wrong_esb_msg(ServiceBus::Ctl, &wrong_msg));
            }
        }

        Ok(())
    }

    fn untrack(&mut self, txid: Txid) {
        debug!("Stopping tracking tx {txid}");
        if self.track_list.remove(&txid).is_none() {
            warn!("Transaction {} was not tracked before", txid);
        }
    }
}

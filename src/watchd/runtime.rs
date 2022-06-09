// LNP Node: node running lightning network protocol and generalized lightning
// channels.
// Written in 2020-2022 by
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

use std::collections::HashMap;

use bitcoin::Txid;
use electrum_client::Client as ElectrumClient;
use lnp::p2p::bolt::Messages as LnMsg;
use microservices::esb;

use crate::bus::{BusMsg, CtlMsg, ServiceBus};
use crate::rpc::ServiceId;
use crate::{Config, Endpoints, Error, Service};

pub fn run(config: Config) -> Result<(), Error> {
    let electrum =
        ElectrumClient::new(&config.electrum_url).map_err(|_| Error::ElectrumConnectivity)?;

    let runtime = Runtime { electrum, track_list: empty!() };

    Service::run(config, runtime, false)
}

pub struct Runtime {
    electrum: ElectrumClient,

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
                debug!("Tracking status for tx {}", txid);
                self.track_list.insert(txid, (depth, source));
            }

            CtlMsg::Untrack(txid) => {
                debug!("Stopping tracking tx {}", txid);
                if self.track_list.remove(&txid).is_none() {
                    warn!("Transaction {} was not tracked before", txid);
                }
            }

            wrong_msg => {
                error!("Request {} is not supported by the CTL interface", wrong_msg);
                return Err(Error::wrong_esb_msg(ServiceBus::Ctl, &wrong_msg));
            }
        }

        Ok(())
    }
}

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

use bitcoin::secp256k1::{self, Secp256k1};
use internet2::TypedEnum;
use microservices::esb;
use psbt::sign::{MemoryKeyProvider, MemorySigningAccount, SignAll};
use std::fs;

use crate::rpc::{Request, ServiceBus};
use crate::{Config, Error, Service, ServiceId};

pub fn run(config: Config) -> Result<(), Error> {
    let secp = Secp256k1::new();

    let mut wallet_path = config.data_dir.clone();
    wallet_path.push("wallet");
    wallet_path.set_extension("dat");
    let signing_account =
        MemorySigningAccount::read(&secp, fs::File::open(wallet_path)?)?;
    let mut provider = MemoryKeyProvider::with(&secp);
    provider.add_account(signing_account);

    let runtime = Runtime {
        identity: ServiceId::Signer,
        provider,
    };

    Service::run(config, runtime, false)
}

pub struct Runtime<'secp>
where
    Self: 'secp,
{
    identity: ServiceId,
    provider: MemoryKeyProvider<'secp, secp256k1::All>,
}

impl<'secp> esb::Handler<ServiceBus> for Runtime<'secp>
where
    Self: 'secp,
{
    type Request = Request;
    type Address = ServiceId;
    type Error = Error;

    fn identity(&self) -> ServiceId {
        self.identity.clone()
    }

    fn handle(
        &mut self,
        senders: &mut esb::SenderList<ServiceBus, ServiceId>,
        bus: ServiceBus,
        source: ServiceId,
        request: Request,
    ) -> Result<(), Self::Error> {
        match bus {
            ServiceBus::Msg => self.handle_rpc_msg(senders, source, request),
            ServiceBus::Ctl => self.handle_rpc_ctl(senders, source, request),
            _ => {
                Err(Error::NotSupported(ServiceBus::Bridge, request.get_type()))
            }
        }
    }

    fn handle_err(&mut self, _: esb::Error) -> Result<(), esb::Error> {
        // We do nothing and do not propagate error; it's already being reported
        // with `error!` macro by the controller. If we propagate error here
        // this will make whole daemon panic
        Ok(())
    }
}

impl<'secp> Runtime<'secp>
where
    Self: 'secp,
{
    #[inline]
    fn handle_rpc_msg(
        &mut self,
        _senders: &mut esb::SenderList<ServiceBus, ServiceId>,
        _source: ServiceId,
        request: Request,
    ) -> Result<(), Error> {
        return Err(Error::NotSupported(ServiceBus::Msg, request.get_type()));
    }

    fn handle_rpc_ctl(
        &mut self,
        senders: &mut esb::SenderList<ServiceBus, ServiceId>,
        source: ServiceId,
        request: Request,
    ) -> Result<(), Error> {
        match request {
            Request::Sign(mut psbt) => {
                psbt.sign_all(&self.provider)?;
                senders.send_to(
                    ServiceBus::Ctl,
                    self.identity.clone(),
                    source,
                    Request::Signed(psbt),
                )?;
                Ok(())
            }
            _ => {
                error!("Request is not supported by the CTL interface");
                return Err(Error::NotSupported(
                    ServiceBus::Ctl,
                    request.get_type(),
                ));
            }
        }
    }
}

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

use std::fs;

use bitcoin::secp256k1::{self, Secp256k1};
use internet2::TypedEnum;
use microservices::esb;
use psbt::sign::{MemoryKeyProvider, MemorySigningAccount, SignAll};

use crate::i9n::ctl::CtlMsg;
use crate::i9n::{BusMsg, ServiceBus};
use crate::opts::LNP_NODE_MASTER_KEY_FILE;
use crate::{Config, Endpoints, Error, Service, ServiceId};

pub fn run(config: Config) -> Result<(), Error> {
    let secp = Secp256k1::new();
    let runtime = Runtime::with(&secp, &config)?;
    Service::run(config, runtime, false)
}

pub struct Runtime<'secp>
where
    Self: 'secp,
{
    identity: ServiceId,
    provider: MemoryKeyProvider<'secp, secp256k1::All>,
}

impl<'secp> Runtime<'secp>
where
    Self: 'secp,
{
    pub fn with(secp: &'secp Secp256k1<secp256k1::All>, config: &Config) -> Result<Self, Error> {
        Ok(Runtime { identity: ServiceId::Signer, provider: Runtime::provider(secp, config)? })
    }

    fn provider(
        secp: &'secp Secp256k1<secp256k1::All>,
        config: &Config,
    ) -> Result<MemoryKeyProvider<'secp, secp256k1::All>, Error> {
        let mut wallet_path = config.data_dir.clone();
        wallet_path.push(LNP_NODE_MASTER_KEY_FILE);
        let signing_account = MemorySigningAccount::read(&secp, fs::File::open(wallet_path)?)?;
        let mut provider = MemoryKeyProvider::with(&secp);
        provider.add_account(signing_account);
        Ok(provider)
    }
}

impl<'secp> esb::Handler<ServiceBus> for Runtime<'secp>
where
    Self: 'secp,
{
    type Request = BusMsg;
    type Address = ServiceId;
    type Error = Error;

    fn identity(&self) -> ServiceId { self.identity.clone() }

    fn handle(
        &mut self,
        endpoints: &mut Endpoints,
        bus: ServiceBus,
        source: ServiceId,
        message: BusMsg,
    ) -> Result<(), Self::Error> {
        match (bus, message, source) {
            (ServiceBus::Ctl, BusMsg::Ctl(msg), source) => self.handle_ctl(endpoints, source, msg),
            (bus, msg, _) => Err(Error::NotSupported(bus, msg.get_type())),
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
    fn handle_ctl(
        &mut self,
        endpoints: &mut Endpoints,
        source: ServiceId,
        message: CtlMsg,
    ) -> Result<(), Error> {
        match message {
            CtlMsg::Sign(mut psbt) => {
                psbt.sign_all(&self.provider)?;
                endpoints.send_to(
                    ServiceBus::Ctl,
                    self.identity.clone(),
                    source,
                    CtlMsg::Signed(psbt),
                )?;
                Ok(())
            }
            wrong_msg => {
                error!("Request {} is not supported by the CTL interface", wrong_msg);
                return Err(Error::NotSupported(ServiceBus::Ctl, wrong_msg.get_type()));
            }
        }
    }
}

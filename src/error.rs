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

use std::io;

use amplify::IoError;
use bitcoin::util::bip32;
use internet2::{presentation, transport};
use lnp::router;
use microservices::esb;
use psbt::sign::SignError;

use crate::bus::ServiceBus;
use crate::channeld;
use crate::lnpd::automata::launch;
use crate::lnpd::{funding, Daemon, DaemonError};
use crate::routed::PaymentError;
use crate::rpc::{self, ServiceId};

#[derive(Debug, Display, From, Error)]
#[display(doc_comments)]
#[non_exhaustive]
pub enum Error {
    /// I/O error: {0:?}
    #[from(io::Error)]
    Io(IoError),

    /// ESB error: {0}
    #[from]
    Esb(esb::Error<ServiceId>),

    /// RPC error: {0}
    #[from]
    Rpc(rpc::Error),

    /// failed to launch a daemon: {0}
    #[from(DaemonError<Daemon>)]
    DaemonLaunch(Box<DaemonError<Daemon>>),

    /// peer interface error: {0}
    #[from]
    Peer(presentation::Error),

    /// channel operations failure: {0}
    #[from]
    #[from(lnp::channel::bolt::Error)]
    Channel(channeld::Error),

    /// router failure: {0}
    #[from]
    GossipRouter(router::gossip::Error),

    /// failed to bootstrap channel: {0}
    #[from]
    ChannelLaunch(launch::Error),

    /// failed to perform the payment. Details: {0}
    #[from]
    Payment(PaymentError),

    /// failing to restore channel state. Details: {0}
    Persistence(strict_encoding::Error),

    /// encoding failure
    ///
    /// Details: {0}
    #[from]
    BitcoinEncoding(bitcoin::consensus::encode::Error),

    /// Error during funding wallet operation
    #[from]
    #[display(inner)]
    FundingWallet(funding::Error),

    /// unable to deriving keys: {0}
    #[from]
    Derivation(bip32::Error),

    /// descriptor failure: {0}
    #[from]
    Miniscript(miniscript::Error),

    /// unable to sign PSBT: {0}
    #[from]
    Signing(SignError),

    /// bridge interface failure: {0}
    #[from(zmq::Error)]
    #[from]
    Bridge(transport::Error),

    /// unable to connect Electrum server
    ElectrumConnectivity,

    /// message `{1}` is not supported on {0} message bus
    NotSupported(ServiceBus, String),

    /// message `{1}` is not supported on {0} message bus for service {2}
    SourceNotSupported(ServiceBus, String, ServiceId),

    /// peer has misbehaved LN peer protocol rules
    Misbehaving,

    /// unrecoverable error "{0}"
    Terminate(String),

    /// other error type with string explanation
    #[display(inner)]
    #[from(internet2::addr::NoOnionSupportError)]
    Other(String),
}

impl microservices::error::Error for Error {}

impl From<Error> for esb::Error<ServiceId> {
    fn from(err: Error) -> Self {
        match err {
            Error::Esb(err) => err,
            err => esb::Error::ServiceError(err.to_string()),
        }
    }
}

impl From<Error> for microservices::rpc::Error {
    fn from(err: Error) -> Self {
        match err {
            Error::Rpc(err) => err.into(),
            err => microservices::rpc::Error::ServerFailure(microservices::rpc::Failure {
                code: 2000,
                info: err.to_string(),
            }),
        }
    }
}

impl Error {
    pub fn wrong_esb_msg(bus: ServiceBus, message: &impl ToString) -> Error {
        Error::NotSupported(bus, message.to_string())
    }

    pub fn wrong_esb_msg_source(
        bus: ServiceBus,
        message: &impl ToString,
        source: ServiceId,
    ) -> Error {
        Error::SourceNotSupported(bus, message.to_string(), source)
    }
}

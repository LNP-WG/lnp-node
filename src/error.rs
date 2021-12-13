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

use std::io;

use amplify::IoError;
use bitcoin::util::bip32;
#[cfg(feature = "_rpc")]
use internet2::TypeId;
use internet2::{presentation, transport};
#[cfg(feature = "_rpc")]
use microservices::{esb, rpc};
use psbt::sign::SignError;

use crate::channeld;
#[cfg(feature = "_rpc")]
use crate::i9n::ServiceBus;
use crate::lnpd::state_machines::channel_launch;
use crate::lnpd::{funding_wallet, Daemon, DaemonError};

#[derive(Debug, Display, From, Error)]
#[display(doc_comments)]
#[non_exhaustive]
pub enum Error {
    /// I/O error: {0:?}
    #[from(io::Error)]
    Io(IoError),

    /// ESB error: {0}
    #[cfg(feature = "_rpc")]
    #[from]
    Esb(esb::Error),

    /// RPC error: {0}
    #[cfg(feature = "_rpc")]
    #[from]
    Rpc(rpc::Error),

    /// Error launching the daemon: {0}
    #[from(DaemonError<Daemon>)]
    DaemonLaunch(Box<DaemonError<Daemon>>),

    /// Peer interface error: {0}
    #[from]
    Peer(presentation::Error),

    /// Channel operations error: {0}
    #[from]
    Channel(channeld::Error),

    /// Error launching channel daemon: {0}
    #[from]
    ChannelLaunch(channel_launch::Error),

    /// Encoding error: {0}
    #[from]
    BitcoinEncoding(bitcoin::consensus::encode::Error),

    /// Error during funding wallet operation
    #[from]
    #[display(inner)]
    FundingWallet(funding_wallet::Error),

    /// Error deriving keys: {0}
    #[from]
    Derivation(bip32::Error),

    /// Error constructing descriptor: {0}
    #[from]
    Miniscript(miniscript::Error),

    /// Error signing PSBT: {0}
    #[from]
    Signing(SignError),

    /// Bridge interface error: {0}
    #[cfg(any(feature = "node", feature = "client"))]
    #[from(zmq::Error)]
    #[from]
    Bridge(transport::Error),

    /// Provided RPC request is not supported for the used type of endpoint
    #[cfg(feature = "_rpc")]
    NotSupported(ServiceBus, TypeId),

    /// Peer does not respond to ping messages
    NotResponding,

    /// Peer has misbehaved LN peer protocol rules
    Misbehaving,

    /// unrecoverable error "{0}"
    Terminate(String),

    /// Other error type with string explanation
    #[display(inner)]
    #[from(internet2::addr::NoOnionSupportError)]
    Other(String),
}

impl microservices::error::Error for Error {}

#[cfg(feature = "_rpc")]
impl From<Error> for esb::Error {
    fn from(err: Error) -> Self {
        match err {
            Error::Esb(err) => err,
            err => esb::Error::ServiceError(err.to_string()),
        }
    }
}

#[cfg(feature = "_rpc")]
impl From<Error> for rpc::Error {
    fn from(err: Error) -> Self {
        match err {
            Error::Rpc(err) => err,
            err => rpc::Error::ServerFailure(rpc::Failure { code: 2000, info: err.to_string() }),
        }
    }
}

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

#[cfg(any(feature = "node", feature = "client"))]
use lnpbp::lnp::TypeId;
use lnpbp::lnp::{presentation, transport};
#[cfg(any(feature = "node", feature = "client"))]
use lnpbp_services::{esb, rpc};

#[cfg(any(feature = "node", feature = "client"))]
use crate::rpc::ServiceBus;

#[derive(Clone, Debug, Display, From, Error)]
#[display(doc_comments)]
#[non_exhaustive]
pub enum Error {
    /// I/O error: {0:?}
    #[from]
    Io(io::ErrorKind),

    /// ESB error: {0}
    #[cfg(any(feature = "node", feature = "client"))]
    #[from]
    Esb(esb::Error),

    /// RPC error: {0}
    #[cfg(any(feature = "node", feature = "client"))]
    #[from]
    Rpc(rpc::Error),

    /// Peer interface error: {0}
    #[from]
    Peer(presentation::Error),

    /// Bridge interface error: {0}
    #[cfg(any(feature = "node", feature = "client"))]
    #[from(zmq::Error)]
    #[from]
    Bridge(transport::Error),

    /// Provided RPC request is not supported for the used type of endpoint
    #[cfg(any(feature = "node", feature = "client"))]
    NotSupported(ServiceBus, TypeId),

    /// Peer does not respond to ping messages
    NotResponding,

    /// Peer has misbehaved LN peer protocol rules
    Misbehaving,

    /// unrecoverable error "{0}"
    Terminate(String),

    /// Other error type with string explanation
    #[display(inner)]
    #[from(amplify::internet::NoOnionSupportError)]
    Other(String),
}

impl lnpbp_services::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::Io(err.kind())
    }
}

#[cfg(any(feature = "node", feature = "client"))]
impl From<Error> for esb::Error {
    fn from(err: Error) -> Self {
        match err {
            Error::Esb(err) => err,
            err => esb::Error::ServiceError(err.to_string()),
        }
    }
}

#[cfg(any(feature = "node", feature = "client"))]
impl From<Error> for rpc::Error {
    fn from(err: Error) -> Self {
        match err {
            Error::Rpc(err) => err,
            err => rpc::Error::ServerFailure(rpc::Failure {
                code: 2000,
                info: err.to_string(),
            }),
        }
    }
}

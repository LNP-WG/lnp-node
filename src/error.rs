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
use lnpbp_services::rpc;

#[cfg(any(feature = "node", feature = "client"))]
use crate::rpc::Endpoints;

#[derive(Clone, Debug, Display, From, Error)]
#[display(doc_comments)]
#[non_exhaustive]
pub enum Error {
    /// I/O error: {_0:?}
    #[from]
    Io(io::ErrorKind),

    /// ZeroMQ error: {_0}
    #[from]
    #[cfg(feature = "zmq")]
    Zmq(zmq::Error),

    /// LNP transport-level error: {_0}
    #[from]
    Transport(transport::Error),

    /// LNP presentation-level error: {_0}
    Presentation(presentation::Error),

    /// RPC error: {_0}
    #[cfg(any(feature = "node", feature = "client"))]
    Rpc(rpc::Error),

    /// Provided RPC request is not supported for the used type of endpoint
    #[cfg(any(feature = "node", feature = "client"))]
    NotSupported(Endpoints, TypeId),

    /// Peer does not respond to ping messages
    NotResponding,

    /// Peer has misbehaved LN peer protocol rules
    Misbehaving,

    /// {_0}
    Other(String),
}

impl lnpbp_services::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::Io(err.kind())
    }
}

impl From<presentation::Error> for Error {
    fn from(err: presentation::Error) -> Self {
        match err {
            presentation::Error::Transport(err) => Error::from(err),
            err => Error::Presentation(err),
        }
    }
}

#[cfg(any(feature = "node", feature = "client"))]
impl From<rpc::Error> for Error {
    fn from(err: rpc::Error) -> Self {
        match err {
            rpc::Error::Transport(err) => Error::from(err),
            rpc::Error::Presentation(err) => Error::from(err),
            rpc::Error::Zmq(err) => Error::Zmq(err),
            err => Error::Rpc(err),
        }
    }
}

#[cfg(any(feature = "node", feature = "client"))]
impl From<Error> for rpc::Error {
    fn from(err: Error) -> Self {
        match err {
            Error::Zmq(err) => rpc::Error::Zmq(err),
            Error::Transport(err) => rpc::Error::Transport(err),
            Error::Presentation(err) => rpc::Error::Presentation(err),
            Error::Rpc(err) => err,
            err => rpc::Error::ServerFailure(rpc::Failure {
                code: 2000,
                info: err.to_string(),
            }),
        }
    }
}

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

#[cfg(any(feature = "node", feature = "client"))]
use lnpbp::lnp::TypeId;

#[cfg(any(feature = "node", feature = "client"))]
use crate::rpc::Endpoints;

#[derive(Clone, Debug, Display, From, Error)]
#[display(doc_comments)]
#[non_exhaustive]
pub enum Error {
    /// ZeroMQ error: {_0}
    #[from]
    #[cfg(feature = "zmq")]
    Zmq(zmq::Error),

    /// This error can't happen
    #[from]
    #[cfg(feature = "node")]
    Server(lnpbp_services::rpc::Error),

    /// LNP transport-level error: {_0}
    #[from]
    Transport(lnpbp::lnp::transport::Error),

    /// LNP presentation-level error: {_0}
    #[from]
    Presentation(lnpbp::lnp::presentation::Error),

    /// Provided RPC request is not supported for the used type of endpoint
    #[cfg(any(feature = "node", feature = "client"))]
    NotSupported(Endpoints, TypeId),
}

impl lnpbp_services::error::Error for Error {}

// LNP Node: node running lightning network protocol and generalized lightning
// channels.
// Written in 2020-2024 by
//     Dr. Maxim Orlovsky <orlovsky@lnp-bp.org>
//
// To the extent possible under law, the author(s) have dedicated all
// copyright and related and neighboring rights to this software to
// the public domain worldwide. This software is distributed without
// any warranty.
//
// You should have received a copy of the MIT License along with this software.
// If not, see <https://opensource.org/licenses/MIT>.

use microservices::{esb, rpc};

use crate::{Failure, ServiceId};

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug, Display)]
#[derive(StrictEncode, StrictDecode)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(crate = "serde_crate"))]
#[display(Debug)]
pub enum FailureCode {
    /// Catch-all
    Unknown = 0xFFF,

    /// Encoding
    Encoding = 0x02,

    /// Launching service
    Launch = 0x03,

    /// Channel error
    Channel = 0x020,

    /// LNPD-related error
    Lnpd = 0x010,

    /// Error coming from other ESB interface reported to a different sservice
    Nested = 0xFFE,
}

impl From<u16> for FailureCode {
    fn from(value: u16) -> Self {
        match value {
            _ => FailureCode::Unknown,
        }
    }
}

impl From<FailureCode> for u16 {
    fn from(code: FailureCode) -> Self { code as u16 }
}

impl From<FailureCode> for rpc::FailureCode<FailureCode> {
    fn from(code: FailureCode) -> Self { rpc::FailureCode::Other(code) }
}

impl rpc::FailureCodeExt for FailureCode {}

#[derive(Debug, Display, From, Error)]
#[display(doc_comments)]
#[non_exhaustive]
pub enum Error {
    /// ESB error: {0}
    #[from]
    Esb(esb::Error<ServiceId>),

    /// RPC error: {0}
    #[from]
    Rpc(rpc::ServerError<FailureCode>),

    /// other error type with string explanation
    #[display(inner)]
    #[from(internet2::addr::NoOnionSupportError)]
    Other(String),
}

impl From<Error> for esb::Error<ServiceId> {
    fn from(err: Error) -> Self {
        match err {
            Error::Esb(err) => err,
            err => esb::Error::ServiceError(err.to_string()),
        }
    }
}

impl From<&esb::Error<ServiceId>> for Failure {
    fn from(err: &esb::Error<ServiceId>) -> Self {
        Failure { code: FailureCode::Nested, info: err.to_string() }
    }
}

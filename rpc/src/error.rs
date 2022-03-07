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

use microservices::{esb, rpc};

use crate::ServiceId;

#[derive(Debug, Display, From, Error)]
#[display(doc_comments)]
#[non_exhaustive]
pub enum Error {
    /// ESB error: {0}
    #[from]
    Esb(esb::Error<ServiceId>),

    /// RPC error: {0}
    #[from]
    Rpc(rpc::Error),

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

impl From<Error> for rpc::Error {
    fn from(err: Error) -> Self {
        match err {
            Error::Rpc(err) => err,
            err => rpc::Error::ServerFailure(rpc::Failure { code: 2000, info: err.to_string() }),
        }
    }
}

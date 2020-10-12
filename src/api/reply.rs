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

//use lnpbp::bitcoin::util::bip32::{ExtendedPrivKey, ExtendedPubKey};
use lnpbp::lnp::presentation::Error;

#[cfg(feature = "daemon")]
use crate::error::RuntimeError;

#[derive(Clone, Debug, Display, LnpApi)]
#[lnp_api(encoding = "strict")]
#[display(Debug)]
#[non_exhaustive]
pub enum Reply {
    #[lnp_api(type = 0x0100)]
    Success,

    #[lnp_api(type = 0x0102)]
    Failure(crate::api::message::Failure),
}

impl From<Error> for Reply {
    fn from(err: Error) -> Self {
        // TODO: Save error code taken from `Error::to_value()` after
        //       implementation of `ToValue` trait and derive macro for enums
        Reply::Failure(crate::api::message::Failure {
            code: 0,
            info: format!("{}", err),
        })
    }
}

#[cfg(feature = "daemon")]
impl From<RuntimeError> for Reply {
    fn from(err: RuntimeError) -> Self {
        // TODO: Save error code taken from `Error::to_value()` after
        //       implementation of `ToValue` trait and derive macro for enums
        Reply::Failure(crate::api::message::Failure {
            code: 0,
            info: format!("{}", err),
        })
    }
}

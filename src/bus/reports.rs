// LNP Node: node running lightning network protocol and generalized lightning
// channels.
// Written in 2020-2022 by
//     Dr. Maxim Orlovsky <orlovsky@lnp-bp.org>
//
// To the extent possible under law, the author(s) have dedicated all
// copyright and related and neighboring rights to this software to
// the public domain worldwide. This software is distributed without
// any warranty.
//
// You should have received a copy of the MIT License along with this software.
// If not, see <https://opensource.org/licenses/MIT>.

use lnp_rpc::{Failure, FailureCode, OptionDetails, RpcMsg};
use microservices::LauncherError;

use crate::lnpd::Daemon;

pub trait ToProgressOrFalure {
    fn to_progress_or_failure(&self) -> RpcMsg;
}
pub trait IntoSuccessOrFalure {
    fn into_success_or_failure(self) -> RpcMsg;
}

impl ToProgressOrFalure for Result<String, LauncherError<Daemon>> {
    fn to_progress_or_failure(&self) -> RpcMsg {
        match self {
            Ok(val) => RpcMsg::Progress(val.clone()),
            Err(err) => RpcMsg::from(Failure { code: FailureCode::Launch, info: err.to_string() }),
        }
    }
}

impl IntoSuccessOrFalure for Result<String, LauncherError<Daemon>> {
    fn into_success_or_failure(self) -> RpcMsg {
        match self {
            Ok(val) => RpcMsg::Success(OptionDetails::with(val)),
            Err(err) => RpcMsg::from(Failure { code: FailureCode::Launch, info: err.to_string() }),
        }
    }
}

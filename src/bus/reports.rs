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

use rpc::{Failure, OptionDetails, RpcMsg};

pub trait ToProgressOrFalure {
    fn to_progress_or_failure(&self) -> RpcMsg;
}
pub trait IntoSuccessOrFalure {
    fn into_success_or_failure(self) -> RpcMsg;
}

impl<E> ToProgressOrFalure for Result<String, E>
where
    E: std::error::Error,
{
    fn to_progress_or_failure(&self) -> RpcMsg {
        match self {
            Ok(val) => RpcMsg::Progress(val.clone()),
            Err(err) => RpcMsg::Failure(Failure::from(err)),
        }
    }
}

impl IntoSuccessOrFalure for Result<String, crate::Error> {
    fn into_success_or_failure(self) -> RpcMsg {
        match self {
            Ok(val) => RpcMsg::Success(OptionDetails::with(val)),
            Err(err) => RpcMsg::from(Failure::from(&err)),
        }
    }
}

impl IntoSuccessOrFalure for Result<(), crate::Error> {
    fn into_success_or_failure(self) -> RpcMsg {
        match self {
            Ok(_) => RpcMsg::Success(OptionDetails::new()),
            Err(err) => RpcMsg::from(Failure::from(&err)),
        }
    }
}

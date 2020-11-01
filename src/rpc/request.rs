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

use amplify::Wrapper;
use std::fmt::{self, Display, Formatter};

use lnpbp::lnp::{
    message, rpc_connection, Invoice, Messages, NodeAddr, RemoteSocketAddr,
};
use lnpbp_services::rpc::Failure;

use crate::ServiceId;

#[derive(Clone, Debug, Display, From, LnpApi)]
#[lnp_api(encoding = "strict")]
#[non_exhaustive]
pub enum Request {
    #[lnp_api(type = 0)]
    #[display("hello()")]
    Hello,

    #[lnp_api(type = 1)]
    #[display("lnpwp({_0})")]
    LnpwpMessage(Messages),

    // Can be issued from `cli` to `lnpd`
    #[lnp_api(type = 2)]
    #[display("listen({_0})")]
    Listen(RemoteSocketAddr),

    // Can be issued from `cli` to `lnpd`
    #[lnp_api(type = 3)]
    #[display("connect({_0})")]
    ConnectPeer(NodeAddr),

    // Can be issued from `cli` to a specific `peerd`
    #[lnp_api(type = 4)]
    #[display("ping_peer()")]
    PingPeer,

    // Can be issued from `cli` to `lnpd`
    #[lnp_api(type = 5)]
    #[display("create_channel_with(...)")]
    OpenChannelWith(ChannelParams),

    #[lnp_api(type = 6)]
    #[display("accept_channel_from(...)")]
    AcceptChannelFrom(ChannelParams),

    // Can be issued from `cli` to a specific `peerd`
    #[lnp_api(type = 7)]
    #[display("pay_invoice({_0})")]
    PayInvoice(Invoice),

    // Responses to CLI
    // ----------------
    #[lnp_api(type = 102)]
    #[display("in_progress({_0})")]
    InProgress(String),

    #[lnp_api(type = 101)]
    #[display("success({_0})")]
    Success(OptionDetails),

    #[lnp_api(type = 100)]
    #[display("failure({_0:#})")]
    #[from]
    Failure(Failure),
}

impl rpc_connection::Request for Request {}

#[derive(
    Wrapper,
    Clone,
    PartialEq,
    Eq,
    Debug,
    From,
    Default,
    StrictEncode,
    StrictDecode,
)]
pub struct OptionDetails(Option<String>);

impl Display for OptionDetails {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.as_inner() {
            None => Ok(()),
            Some(msg) => f.write_str(&msg),
        }
    }
}

impl OptionDetails {
    pub fn with(s: impl ToString) -> Self {
        Self(Some(s.to_string()))
    }

    pub fn new() -> Self {
        Self(None)
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Display, StrictEncode, StrictDecode)]
#[display("{peerd}, ...")]
pub struct ChannelParams {
    pub channel_req: message::OpenChannel,
    pub peerd: ServiceId,
}

impl From<crate::Error> for Request {
    fn from(err: crate::Error) -> Self {
        Request::Failure(Failure::from(err))
    }
}

pub trait IntoProcessOrFalure {
    fn into_process_or_failure(self) -> Request;
}
pub trait IntoSuccessOrFalure {
    fn into_success_or_failure(self) -> Request;
}

impl IntoProcessOrFalure for Result<String, crate::Error> {
    fn into_process_or_failure(self) -> Request {
        match self {
            Ok(val) => Request::InProgress(val),
            Err(err) => Request::from(err),
        }
    }
}

impl IntoSuccessOrFalure for Result<String, crate::Error> {
    fn into_success_or_failure(self) -> Request {
        match self {
            Ok(val) => Request::Success(OptionDetails::with(val)),
            Err(err) => Request::from(err),
        }
    }
}

impl IntoSuccessOrFalure for Result<(), crate::Error> {
    fn into_success_or_failure(self) -> Request {
        match self {
            Ok(_) => Request::Success(OptionDetails::new()),
            Err(err) => Request::from(err),
        }
    }
}

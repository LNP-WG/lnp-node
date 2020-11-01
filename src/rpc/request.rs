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
use std::fmt::{self, Debug, Display, Formatter};
use std::time::Duration;

use lnpbp::bitcoin::{secp256k1, Txid};
use lnpbp::bp::chain::AssetId;
use lnpbp::lnp::{
    message, rpc_connection, ChannelId, ChannelState, Invoice, Messages,
    NodeAddr, RemoteSocketAddr,
};
use lnpbp::strict_encoding::{self, StrictDecode, StrictEncode};
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
    #[display("send_message({0})")]
    SendMessage(Messages),

    // Can be issued from `cli` to `lnpd`
    #[lnp_api(type = 100)]
    #[display("node_info()")]
    GetInfo,

    // Can be issued from `cli` to `lnpd`
    #[lnp_api(type = 101)]
    #[display("list_peers()")]
    ListPeers,

    // Can be issued from `cli` to `lnpd`
    #[lnp_api(type = 102)]
    #[display("list_channels()")]
    ListChannels,

    // Can be issued from `cli` to `lnpd`
    #[lnp_api(type = 200)]
    #[display("listen({0})")]
    Listen(RemoteSocketAddr),

    // Can be issued from `cli` to `lnpd`
    #[lnp_api(type = 201)]
    #[display("connect({0})")]
    ConnectPeer(NodeAddr),

    // Can be issued from `cli` to a specific `peerd`
    #[lnp_api(type = 202)]
    #[display("ping_peer()")]
    PingPeer,

    // Can be issued from `cli` to `lnpd`
    #[lnp_api(type = 203)]
    #[display("create_channel_with(...)")]
    OpenChannelWith(CreateChannel),

    #[lnp_api(type = 204)]
    #[display("accept_channel_from(...)")]
    AcceptChannelFrom(CreateChannel),

    // Can be issued from `cli` to a specific `peerd`
    #[lnp_api(type = 205)]
    #[display("pay_invoice({0})")]
    PayInvoice(Invoice),

    // Responses to CLI
    // ----------------
    #[lnp_api(type = 1002)]
    #[display("progress({0})")]
    Progress(String),

    #[lnp_api(type = 1001)]
    #[display("success({0})")]
    Success(OptionDetails),

    #[lnp_api(type = 1000)]
    #[display("failure({0:#})")]
    #[from]
    Failure(Failure),

    #[lnp_api(type = 1100)]
    #[display("nonde_info({0})", alt = "{0:#}")]
    #[from]
    NodeInfo(NodeInfo),

    #[lnp_api(type = 1101)]
    #[display("peer_list({0})", alt = "{0:#}")]
    #[from]
    PeerList(List<PeerInfo>),

    #[lnp_api(type = 1102)]
    #[display("channel_list({0})", alt = "{0:#}")]
    #[from]
    ChannelList(List<ChannelInfo>),
}

impl rpc_connection::Request for Request {}

// TODO: Move to amplify
#[cfg(feature = "serde")]
pub trait ToYamlString
where
    Self: serde::Serialize,
{
    fn to_yaml_string(&self) -> String {
        serde_yaml::to_string(self).expect("internal YAML serialization error")
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Display, StrictEncode, StrictDecode)]
#[display("{peerd}, ...")]
pub struct CreateChannel {
    pub channel_req: message::OpenChannel,
    pub peerd: ServiceId,
    pub report_to: Option<ServiceId>,
}

#[derive(Clone, PartialEq, Eq, Debug, Display, StrictEncode, StrictDecode)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(crate = "serde_crate")
)]
#[display(NodeInfo::to_yaml_string)]
pub struct NodeInfo {
    pub node_id: secp256k1::PublicKey,
    pub listens: Vec<RemoteSocketAddr>,
    pub uptime: Duration,
    pub since: u64,
    pub peers: usize,
    pub channels: usize,
}

#[derive(Clone, PartialEq, Eq, Debug, Display, StrictEncode, StrictDecode)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(crate = "serde_crate")
)]
#[display(PeerInfo::to_yaml_string)]
pub struct PeerInfo {
    pub node_id: secp256k1::PublicKey,
    pub uptime: Duration,
    pub since: i64,
    pub channels: usize,
}

#[derive(Clone, PartialEq, Eq, Debug, Display, StrictEncode, StrictDecode)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(crate = "serde_crate")
)]
#[display(ChannelInfo::to_yaml_string)]
pub struct ChannelInfo {
    pub channel_id: ChannelId,
    pub state: ChannelState,
    pub capacities: (u64, u64),
    pub assets: Vec<AssetId>,
    // assets: HashMap<AssetId, ((u64, u64), String)>,
    pub funding_tx: Txid,
    pub remote_peers: Vec<NodeAddr>,
    pub uptime: Duration,
    pub since: i64,
    pub total_updates: u64,
    pub pending_updates: u16,
    pub max_updates: u16,
}

#[cfg(feature = "serde")]
impl ToYamlString for NodeInfo {}
#[cfg(feature = "serde")]
impl ToYamlString for PeerInfo {}
#[cfg(feature = "serde")]
impl ToYamlString for ChannelInfo {}

#[derive(
    Wrapper, Clone, PartialEq, Eq, Debug, From, StrictEncode, StrictDecode,
)]
#[wrapper(IndexRange)]
pub struct List<T>(Vec<T>)
where
    T: Clone
        + PartialEq
        + Eq
        + Debug
        + Display
        + StrictEncode<Error = strict_encoding::Error>
        + StrictDecode<Error = strict_encoding::Error>;

#[cfg(feature = "serde")]
impl<'a, T> Display for List<T>
where
    T: Clone
        + PartialEq
        + Eq
        + Debug
        + Display
        + serde::Serialize
        + StrictEncode<Error = strict_encoding::Error>
        + StrictDecode<Error = strict_encoding::Error>,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(
            &serde_yaml::to_string(self)
                .expect("internal YAML serialization error"),
        )
    }
}

#[cfg(feature = "serde")]
impl<T> serde::Serialize for List<T>
where
    T: Clone
        + PartialEq
        + Eq
        + Debug
        + Display
        + serde::Serialize
        + StrictEncode<Error = strict_encoding::Error>
        + StrictDecode<Error = strict_encoding::Error>,
{
    fn serialize<S>(
        &self,
        serializer: S,
    ) -> Result<<S as serde::Serializer>::Ok, <S as serde::Serializer>::Error>
    where
        S: serde::Serializer,
    {
        self.as_inner().serialize(serializer)
    }
}

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
pub struct OptionDetails(pub Option<String>);

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

impl From<crate::Error> for Request {
    fn from(err: crate::Error) -> Self {
        Request::Failure(Failure::from(err))
    }
}

pub trait IntoProgressOrFalure {
    fn into_progress_or_failure(self) -> Request;
}
pub trait IntoSuccessOrFalure {
    fn into_success_or_failure(self) -> Request;
}

impl IntoProgressOrFalure for Result<String, crate::Error> {
    fn into_progress_or_failure(self) -> Request {
        match self {
            Ok(val) => Request::Progress(val),
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

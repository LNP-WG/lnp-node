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

use std::collections::BTreeMap;
use std::fmt::{self, Debug, Display, Formatter};
use std::iter::FromIterator;
use std::time::Duration;

use amplify::{ToYamlString, Wrapper};
use bitcoin::{secp256k1, Address, OutPoint};
use internet2::addr::InetSocketAddr;
use internet2::{NodeAddr, RemoteNodeAddr, RemoteSocketAddr};
use lnp::bolt::{self, AssetsBalance, CommonParams, Lifecycle, PeerParams};
use lnp::p2p::legacy::{ChannelId, ChannelType, TempChannelId};
use lnpbp::chain::AssetId;
use microservices::rpc_connection;
#[cfg(feature = "rgb")]
use rgb::Consignment;
#[cfg(feature = "serde")]
use serde_with::{DisplayFromStr, DurationSeconds, Same};
use strict_encoding::{StrictDecode, StrictEncode};
use wallet::address::AddressCompat;

use crate::ServiceId;

/// RPC API requests between LNP Node daemons and clients.
#[derive(Clone, Debug, Display, From, Api)]
#[derive(NetworkEncode, NetworkDecode)]
#[api(encoding = "strict")]
#[non_exhaustive]
pub enum RpcMsg {
    #[api(type = 100)]
    #[display("get_info()")]
    GetInfo,

    #[api(type = 101)]
    #[display("list_peers()")]
    ListPeers,

    #[api(type = 102)]
    #[display("list_channels()")]
    ListChannels,

    #[api(type = 103)]
    #[display("list_funds()")]
    ListFunds,

    #[api(type = 200)]
    #[display("listen({0})")]
    Listen(RemoteSocketAddr),

    // Node connectivity API
    // ---------------------
    #[api(type = 201)]
    #[display("connect({0})")]
    ConnectPeer(RemoteNodeAddr),

    #[api(type = 202)]
    #[display("ping_peer()")]
    PingPeer,

    // Channel API
    // -----------
    /// Requests creation of a new outbound channel by a client.
    #[api(type = 203)]
    #[display("create_channel({0})")]
    CreateChannel(CreateChannel),

    // Can be issued from `cli` to a specific `peerd`
    #[cfg(feature = "rgb")]
    #[api(type = 401)]
    #[display("refill_channel({0})")]
    RefillChannel(RefillChannel),

    // Can be issued from `cli` to a specific `peerd`
    #[api(type = 402)]
    #[display("transfer({0})")]
    Transfer(Transfer),

    /* TODO: Activate after lightning-invoice library update
    // Can be issued from `cli` to a specific `peerd`
    #[api(type = 208)]
    #[display("pay_invoice({0})")]
    PayInvoice(Invoice),
     */
    // Responses to CLI
    // ----------------
    #[api(type = 1002)]
    #[display("progress(\"{0}\")")]
    #[from]
    Progress(String),

    #[api(type = 1001)]
    #[display("success({0})")]
    Success(OptionDetails),

    #[api(type = 1000)]
    #[display("failure({0:#})")]
    #[from]
    Failure(Failure),

    #[api(type = 1100)]
    #[display("node_info({0})", alt = "{0:#}")]
    NodeInfo(NodeInfo),

    #[api(type = 1101)]
    #[display("node_info({0})", alt = "{0:#}")]
    PeerInfo(PeerInfo),

    #[api(type = 1102)]
    #[display("channel_info({0})", alt = "{0:#}")]
    ChannelInfo(ChannelInfo),

    #[api(type = 1103)]
    #[display("peer_list({0})", alt = "{0:#}")]
    PeerList(List<NodeAddr>),

    #[api(type = 1104)]
    #[display("channel_list({0})", alt = "{0:#}")]
    ChannelList(List<ChannelId>),

    #[api(type = 1105)]
    #[display("funds_info({0})", alt = "{0:#}")]
    FundsInfo(FundsInfo),
}

impl rpc_connection::Request for RpcMsg {}

/// Request to create channel originating from a client
#[derive(Clone, PartialEq, Eq, Debug, Display, NetworkEncode, NetworkDecode)]
#[display("{peerd}, {funding_sat}, ...")]
pub struct CreateChannel {
    /// Node to open a channel with
    pub peerd: NodeAddr,

    /// Client identifier to report about the progress
    pub report_to: Option<ServiceId>,

    /// Amount of satoshis for channel funding
    pub funding_sat: u64,

    /// Amount of millisatoshis to pay to the remote peer at the channel opening
    pub push_msat: u64,

    // The following are the customization of the channel parameters which should override node
    // settings
    /// Initial fee rate in satoshi per 1000-weight (i.e. 1/4 the more normally-used 'satoshi
    /// per 1000 vbytes') that this side will pay for commitment and HTLC transactions, as
    /// described in BOLT #3 (this can be adjusted later with an `Fee` message).
    pub fee_rate: Option<u32>,

    /// Should the channel be announced to the lightning network. Required for the node to earn
    /// routing fees. Setting this flag results in the channel and node becoming
    /// public.
    pub announce_channel: Option<bool>,

    /// Channel type as defined in BOLT-2.
    pub channel_type: Option<ChannelType>,

    /// The threshold below which outputs on transactions broadcast by sender will be omitted.
    pub dust_limit: Option<u64>,

    /// The number of blocks which the counterparty will have to wait to claim on-chain funds
    /// if they broadcast a commitment transaction
    pub to_self_delay: Option<u16>,

    /// The maximum number of the received HTLCs.
    pub htlc_max_count: Option<u16>,

    /// Indicates the smallest value of an HTLC this node will accept, in milli-satoshi.
    pub htlc_min_value: Option<u64>,

    /// The maximum inbound HTLC value in flight towards this node, in milli-satoshi
    pub htlc_max_total_value: Option<u64>,

    /// The minimum value unencumbered by HTLCs for the counterparty to keep in
    /// the channel, in satoshis.
    pub channel_reserve: Option<u64>,
}

impl CreateChannel {
    /// Applies customized parameters from the request to a given parameter objects
    pub fn apply_params(&self, common: &mut CommonParams, local: &mut PeerParams) {
        if let Some(fee_rate) = self.fee_rate {
            common.feerate_per_kw = fee_rate;
        }
        if let Some(announce_channel) = self.announce_channel {
            common.announce_channel = announce_channel;
        }
        if let Some(channel_type) = self.channel_type {
            common.channel_type = channel_type;
        }
        if let Some(dust_limit) = self.dust_limit {
            local.dust_limit_satoshis = dust_limit;
        }
        if let Some(to_self_delay) = self.to_self_delay {
            local.to_self_delay = to_self_delay
        }
        if let Some(htlc_max_count) = self.htlc_max_count {
            local.max_accepted_htlcs = htlc_max_count
        }
        if let Some(htlc_min_value) = self.htlc_min_value {
            local.htlc_minimum_msat = htlc_min_value
        }
        if let Some(htlc_max_total_value) = self.htlc_max_total_value {
            local.max_htlc_value_in_flight_msat = htlc_max_total_value
        }
        if let Some(channel_reserve) = self.channel_reserve {
            local.channel_reserve_satoshis = channel_reserve
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Display, NetworkEncode, NetworkDecode)]
#[display("{amount} {asset:?} to {channeld}")]
pub struct Transfer {
    pub channeld: ServiceId,
    pub amount: u64,
    pub asset: Option<AssetId>,
}

#[cfg(feature = "rgb")]
#[derive(Clone, PartialEq, Eq, Debug, Display, NetworkEncode, NetworkDecode)]
#[display("{outpoint}, {blinding}, ...")]
pub struct RefillChannel {
    pub consignment: Consignment,
    pub outpoint: OutPoint,
    pub blinding: u64,
}

#[cfg_attr(feature = "serde", serde_as)]
#[derive(Clone, PartialEq, Eq, Debug, Display, NetworkEncode, NetworkDecode)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(crate = "serde_crate"))]
#[display(NodeInfo::to_yaml_string)]
pub struct NodeInfo {
    pub node_id: secp256k1::PublicKey,
    pub listens: Vec<RemoteSocketAddr>,
    #[serde_as(as = "DurationSeconds")]
    pub uptime: Duration,
    pub since: u64,
    #[serde_as(as = "Vec<DisplayFromStr>")]
    pub peers: Vec<NodeAddr>,
    #[serde_as(as = "Vec<DisplayFromStr>")]
    pub channels: Vec<ChannelId>,
}

#[cfg_attr(feature = "serde", serde_as)]
#[derive(Clone, PartialEq, Eq, Debug, Display, NetworkEncode, NetworkDecode)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(crate = "serde_crate"))]
#[display(PeerInfo::to_yaml_string)]
pub struct PeerInfo {
    pub local_id: secp256k1::PublicKey,
    pub remote_id: Vec<secp256k1::PublicKey>,
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub local_socket: Option<InetSocketAddr>,
    #[serde_as(as = "Vec<DisplayFromStr>")]
    pub remote_socket: Vec<InetSocketAddr>,
    #[serde_as(as = "DurationSeconds")]
    pub uptime: Duration,
    pub since: u64,
    pub messages_sent: usize,
    pub messages_received: usize,
    #[serde_as(as = "Vec<DisplayFromStr>")]
    pub channels: Vec<ChannelId>,
    pub connected: bool,
    pub awaits_pong: bool,
}

pub type RemotePeerMap<T> = BTreeMap<NodeAddr, T>;

//#[serde_as]
#[cfg_attr(feature = "serde", serde_as)]
#[derive(Clone, PartialEq, Eq, Debug, Display, NetworkEncode, NetworkDecode)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(crate = "serde_crate"))]
#[display(ChannelInfo::to_yaml_string)]
pub struct ChannelInfo {
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub channel_id: Option<ChannelId>,
    #[serde_as(as = "DisplayFromStr")]
    pub temporary_channel_id: TempChannelId,
    pub state: Lifecycle,
    pub local_capacity: u64,
    #[serde_as(as = "BTreeMap<DisplayFromStr, Same>")]
    pub remote_capacities: RemotePeerMap<u64>,
    #[serde_as(as = "Vec<DisplayFromStr>")]
    pub assets: Vec<AssetId>,
    #[serde_as(as = "BTreeMap<DisplayFromStr, Same>")]
    pub local_balances: AssetsBalance,
    #[serde_as(as = "BTreeMap<DisplayFromStr, BTreeMap<DisplayFromStr, Same>>")]
    pub remote_balances: RemotePeerMap<AssetsBalance>,
    pub funding_outpoint: OutPoint,
    #[serde_as(as = "Vec<DisplayFromStr>")]
    pub remote_peers: Vec<NodeAddr>,
    #[serde_as(as = "DurationSeconds")]
    pub uptime: Duration,
    pub since: u64,
    pub commitment_updates: u64,
    pub total_payments: u64,
    pub pending_payments: u16,
    pub is_originator: bool,
    pub common_params: bolt::CommonParams,
    pub local_params: bolt::PeerParams,
    pub remote_params: bolt::PeerParams,
    pub local_keys: bolt::Keyset,
    #[serde_as(as = "BTreeMap<DisplayFromStr, Same>")]
    pub remote_keys: BTreeMap<NodeAddr, bolt::Keyset>,
}

#[cfg_attr(feature = "serde", serde_as)]
#[derive(Clone, PartialEq, Eq, Debug, Display, NetworkEncode, NetworkDecode)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(crate = "serde_crate"))]
#[display(FundsInfo::to_yaml_string)]
pub struct FundsInfo {
    #[serde_as(as = "BTreeMap<DisplayFromStr, Same>")]
    pub bitcoin_funds: BTreeMap<AddressCompat, u64>,
    pub asset_funds: AssetsBalance,
    pub next_address: Address,
}

#[cfg(feature = "serde")]
impl ToYamlString for NodeInfo {}
#[cfg(feature = "serde")]
impl ToYamlString for PeerInfo {}
#[cfg(feature = "serde")]
impl ToYamlString for ChannelInfo {}
#[cfg(feature = "serde")]
impl ToYamlString for FundsInfo {}

#[derive(Wrapper, Clone, PartialEq, Eq, Debug, From, NetworkEncode, NetworkDecode)]
#[wrapper(IndexRange)]
pub struct List<T>(Vec<T>)
where
    T: Clone + PartialEq + Eq + Debug + Display + StrictEncode + StrictDecode;

#[cfg(feature = "serde")]
impl<'a, T> Display for List<T>
where
    T: Clone + PartialEq + Eq + Debug + Display + serde::Serialize + StrictEncode + StrictDecode,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&serde_yaml::to_string(self).expect("internal YAML serialization error"))
    }
}

impl<T> FromIterator<T> for List<T>
where
    T: Clone + PartialEq + Eq + Debug + Display + serde::Serialize + StrictEncode + StrictDecode,
{
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self::from_inner(iter.into_iter().collect())
    }
}

#[cfg(feature = "serde")]
impl<T> serde::Serialize for List<T>
where
    T: Clone + PartialEq + Eq + Debug + Display + serde::Serialize + StrictEncode + StrictDecode,
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

/// Information about server-side failure returned through RPC API
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(crate = "serde_crate"))]
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Display)]
#[derive(NetworkEncode, NetworkDecode)]
#[display("{info}", alt = "Server returned failure #{code}: {info}")]
pub struct Failure {
    /// Failure code
    pub code: u16,

    /// Detailed information about the failure
    pub info: String,
}

impl Failure {
    pub fn into_microservice_failure(self) -> microservices::rpc::Failure {
        microservices::rpc::Failure { code: self.code, info: self.info }
    }
}

#[derive(Wrapper, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, From, Default)]
#[derive(NetworkEncode, NetworkDecode)]
pub struct OptionDetails(pub Option<String>);

impl Display for OptionDetails {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.as_inner() {
            None => Ok(()),
            Some(msg) => write!(f, "\"{}\"", msg),
        }
    }
}

impl OptionDetails {
    pub fn with(s: impl ToString) -> Self { Self(Some(s.to_string())) }

    pub fn new() -> Self { Self(None) }
}

impl From<String> for OptionDetails {
    fn from(s: String) -> Self { OptionDetails(Some(s)) }
}

impl From<&str> for OptionDetails {
    fn from(s: &str) -> Self { OptionDetails(Some(s.to_string())) }
}

impl From<crate::Error> for RpcMsg {
    fn from(err: crate::Error) -> Self { RpcMsg::Failure(Failure::from(&err)) }
}

impl From<&str> for RpcMsg {
    fn from(s: &str) -> Self { RpcMsg::Progress(s.to_owned()) }
}

impl<E: std::error::Error> From<&E> for Failure {
    fn from(err: &E) -> Self {
        Failure {
            code: 9000, // TODO: do code types
            info: err.to_string(),
        }
    }
}

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
            Err(err) => RpcMsg::from(err),
        }
    }
}

impl IntoSuccessOrFalure for Result<(), crate::Error> {
    fn into_success_or_failure(self) -> RpcMsg {
        match self {
            Ok(_) => RpcMsg::Success(OptionDetails::new()),
            Err(err) => RpcMsg::from(err),
        }
    }
}

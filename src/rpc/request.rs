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
use bitcoin::{secp256k1, Address, OutPoint, Txid};
use bitcoin_onchain::blockchain::MiningStatus;
use internet2::addr::InetSocketAddr;
use internet2::{NodeAddr, RemoteSocketAddr};
use lnp::p2p::legacy::{ChannelId, Messages, OpenChannel, TempChannelId};
use lnp::payment::{self, AssetsBalance, Lifecycle};
use lnpbp::chain::AssetId;
use microservices::rpc::Failure;
use microservices::rpc_connection;
use psbt::Psbt;
#[cfg(feature = "rgb")]
use rgb::Consignment;
#[cfg(feature = "serde")]
use serde_with::{DisplayFromStr, DurationSeconds, Same};
use strict_encoding::{StrictDecode, StrictEncode};
use wallet::address::AddressCompat;
use wallet::scripts::PubkeyScript;

use crate::ServiceId;

/// RPC API requests over CTL message bus between LNP Node daemons and from/to clients.
#[derive(Clone, Debug, Display, From, Api)]
#[api(encoding = "strict")]
#[non_exhaustive]
pub enum Request {
    #[api(type = 0)]
    #[display("hello()")]
    Hello,

    #[api(type = 1)]
    #[display("update_channel_id({0})")]
    UpdateChannelId(ChannelId),

    #[api(type = 2)]
    #[display("send_message({0})")]
    PeerMessage(Messages),

    // Can be issued from `cli` to `lnpd`
    #[api(type = 100)]
    #[display("get_info()")]
    GetInfo,

    // Can be issued from `cli` to `lnpd`
    #[api(type = 101)]
    #[display("list_peers()")]
    ListPeers,

    // Can be issued from `cli` to `lnpd`
    #[api(type = 102)]
    #[display("list_channels()")]
    ListChannels,

    // Can be issued from `cli` to `lnpd`
    #[api(type = 103)]
    #[display("list_funds()")]
    ListFunds,

    // Can be issued from `cli` to `lnpd`
    #[api(type = 200)]
    #[display("listen({0})")]
    Listen(RemoteSocketAddr),

    // Node connectivity API
    // ---------------------

    // Can be issued from `cli` to `lnpd`
    #[api(type = 201)]
    #[display("connect({0})")]
    ConnectPeer(NodeAddr),

    // Can be issued from `cli` to a specific `peerd`
    #[api(type = 202)]
    #[display("ping_peer()")]
    PingPeer,

    // Channel creation API
    // --------------------
    /// Initiates creation of a new channel by a local node. Sent from clients to lnpd, and then to
    /// a newly instantiated channeld
    #[api(type = 203)]
    #[display("open_channel_with({0})")]
    OpenChannelWith(CreateChannel),

    /// Initiates creation of a new channel by a remote node. Sent from lnpd to a newly
    /// instantiated channeld.
    #[api(type = 204)]
    #[display("accept_channel_from({0})")]
    AcceptChannelFrom(CreateChannel),

    /// Constructs funding transaction PSBT to fund a locally-created new channel. Sent from peerd
    /// to lnpd.
    #[api(type = 205)]
    #[display("construct_funding({0})")]
    ConstructFunding(FundChannel),

    /// Provides channeld with the information about funding transaction output used to fund the
    /// newly created channel. Sent from lnpd to channeld.
    #[api(type = 206)]
    #[display("funding_constructed({0})")]
    FundingConstructed(OutPoint),

    /// Signs previously prepared funding transaction and publishes it to bitcoin network. Sent
    /// from channeld to lnpd upon receival of `funding_sighned` message from a remote peer.
    #[api(type = 207)]
    #[display("publish_funding()")]
    PublishFunding,

    /// Reports back to channeld that the funding transaction was published and its mining status
    /// should be monitored onchain.
    #[api(type = 208)]
    #[display("funding_published()")]
    FundingPublished,

    // On-chain tracking API
    // ---------------------
    /// Asks on-chain tracking service to send updates on the transaction mining status
    #[api(type = 301)]
    #[display("track({0})")]
    Track(Txid),

    /// Asks on-chain tracking service to stop sending updates on the transaction mining status
    #[api(type = 302)]
    #[display("untrack({0})")]
    Untrack(Txid),

    /// Reports changes in the mining status for previously requested transaction tracked by an
    /// on-chain service
    #[api(type = 303)]
    #[display("mined({0})")]
    Mined(MiningInfo),

    // Non-standard API
    // ----------------

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
    #[from]
    NodeInfo(NodeInfo),

    #[api(type = 1101)]
    #[display("node_info({0})", alt = "{0:#}")]
    #[from]
    PeerInfo(PeerInfo),

    #[api(type = 1102)]
    #[display("channel_info({0})", alt = "{0:#}")]
    #[from]
    ChannelInfo(ChannelInfo),

    #[api(type = 1103)]
    #[display("peer_list({0})", alt = "{0:#}")]
    #[from]
    PeerList(List<NodeAddr>),

    #[api(type = 1104)]
    #[display("channel_list({0})", alt = "{0:#}")]
    #[from]
    ChannelList(List<ChannelId>),

    #[api(type = 1105)]
    #[display("funds_info({0})", alt = "{0:#}")]
    #[from]
    FundsInfo(FundsInfo),

    #[api(type = 1203)]
    #[display("channel_funding({0})", alt = "{0:#}")]
    #[from]
    ChannelFunding(PubkeyScript),

    #[api(type = 9000)]
    #[display("sign(...)")]
    Sign(Psbt),

    #[api(type = 9002)]
    #[display("signed(...)")]
    Signed(Psbt),
}

impl rpc_connection::Request for Request {}

#[derive(Clone, PartialEq, Eq, Debug, Display, StrictEncode, StrictDecode)]
#[display("{peerd}, {channel_req}")]
pub struct CreateChannel {
    pub channel_req: OpenChannel,
    pub peerd: NodeAddr,
    pub report_to: Option<ServiceId>,
}

/// Request information about constructing funding transaction
#[derive(Clone, PartialEq, Eq, Debug, Display, StrictEncode, StrictDecode)]
#[display("{address}, {amount}")]
pub struct FundChannel {
    /// Address for the channel funding
    pub address: AddressCompat,

    /// Amount of funds to be sent to the funding address
    pub amount: u64,

    /// Fee to pay for the funding transaction
    pub fee: u64,
}

/// Update on a transaction mining status
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Display)]
#[derive(StrictEncode, StrictDecode)]
#[display("{txid}, {status}")]
pub struct MiningInfo {
    /// Id of a transaction previously requested to be tracked
    pub txid: Txid,

    /// Updated on-chain status of the transaction
    pub status: MiningStatus,
}

#[derive(Clone, PartialEq, Eq, Debug, Display, StrictEncode, StrictDecode)]
#[display("{amount} {asset:?} to {channeld}")]
pub struct Transfer {
    pub channeld: ServiceId,
    pub amount: u64,
    pub asset: Option<AssetId>,
}

#[cfg(feature = "rgb")]
#[derive(Clone, PartialEq, Eq, Debug, Display, StrictEncode, StrictDecode)]
#[display("{outpoint}, {blinding}, ...")]
pub struct RefillChannel {
    pub consignment: Consignment,
    pub outpoint: OutPoint,
    pub blinding: u64,
}

#[cfg_attr(feature = "serde", serde_as)]
#[derive(Clone, PartialEq, Eq, Debug, Display, StrictEncode, StrictDecode)]
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
#[derive(Clone, PartialEq, Eq, Debug, Display, StrictEncode, StrictDecode)]
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
#[derive(Clone, PartialEq, Eq, Debug, Display, StrictEncode, StrictDecode)]
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
    pub params: payment::channel::Params,
    pub local_keys: payment::channel::Keyset,
    #[serde_as(as = "BTreeMap<DisplayFromStr, Same>")]
    pub remote_keys: BTreeMap<NodeAddr, payment::channel::Keyset>,
}

#[cfg_attr(feature = "serde", serde_as)]
#[derive(Clone, PartialEq, Eq, Debug, Display, StrictEncode, StrictDecode)]
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

#[derive(Wrapper, Clone, PartialEq, Eq, Debug, From, StrictEncode, StrictDecode)]
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

#[derive(Wrapper, Clone, PartialEq, Eq, Debug, From, Default, StrictEncode, StrictDecode)]
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

impl From<crate::Error> for Request {
    fn from(err: crate::Error) -> Self { Request::Failure(Failure::from(err)) }
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

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use amplify::hex::FromHex;
    use amplify::DumbDefault;
    use bitcoin::secp256k1;
    use internet2::RemoteNodeAddr;
    use strict_encoding::strict_deserialize;
    use strict_encoding_test::test_encoding_roundtrip;

    use super::*;

    #[test]
    fn strict_encoding() {
        let channel_req = OpenChannel::dumb_default();
        let data = Vec::<u8>::from_hex(
            "000000000000000000000000000000000000000000000000000000000000000000\
            0000000000000000000000000000000000000000000000000000000000000000000\
            0000000000000000000000000000000000000000000000000000000000000000000\
            00000000000000000000000000000000000000000279be667ef9dcbbac55a06295c\
            e870b07029bfcdb2dce28d959f2815b16f817980279be667ef9dcbbac55a06295ce\
            870b07029bfcdb2dce28d959f2815b16f817980279be667ef9dcbbac55a06295ce8\
            70b07029bfcdb2dce28d959f2815b16f817980279be667ef9dcbbac55a06295ce87\
            0b07029bfcdb2dce28d959f2815b16f817980279be667ef9dcbbac55a06295ce870\
            b07029bfcdb2dce28d959f2815b16f817980279be667ef9dcbbac55a06295ce870b\
            07029bfcdb2dce28d959f2815b16f81798000000000000"
        ).unwrap();
        // Checking that the data are entirely consumed
        let _: OpenChannel = strict_deserialize(&data).unwrap();
        test_encoding_roundtrip(&channel_req, data).unwrap();

        let node_id = secp256k1::PublicKey::from_str(
            "022e58afe51f9ed8ad3cc7897f634d881fdbe49a81564629ded8156bebd2ffd1af",
        )
        .unwrap();
        let node_addr = NodeAddr::Remote(RemoteNodeAddr {
            node_id,
            remote_addr: "lnp://127.0.0.1:9735".parse().unwrap(),
        });
        let peerd = ServiceId::Peer(node_addr.clone());
        let data = Vec::<u8>::from_hex(
            "0401022e58afe51f9ed8ad3cc7897f634d881fdbe49a81564629ded8156bebd2ff\
            d1af000000000000000000000000000000000000000000000000000000000000007\
            f000001260700"
        ).unwrap();
        let _: ServiceId = strict_deserialize(&data).unwrap();
        test_encoding_roundtrip(&peerd, data).unwrap();

        let open_channel = CreateChannel { channel_req, peerd: node_addr, report_to: None };

        let data = Vec::<u8>::from_hex(
            "000000000000000000000000000000000000000000000000000000000000000000\
            0000000000000000000000000000000000000000000000000000000000000000000\
            0000000000000000000000000000000000000000000000000000000000000000000\
            00000000000000000000000000000000000000000279be667ef9dcbbac55a06295c\
            e870b07029bfcdb2dce28d959f2815b16f817980279be667ef9dcbbac55a06295ce\
            870b07029bfcdb2dce28d959f2815b16f817980279be667ef9dcbbac55a06295ce8\
            70b07029bfcdb2dce28d959f2815b16f817980279be667ef9dcbbac55a06295ce87\
            0b07029bfcdb2dce28d959f2815b16f817980279be667ef9dcbbac55a06295ce870\
            b07029bfcdb2dce28d959f2815b16f817980279be667ef9dcbbac55a06295ce870b\
            07029bfcdb2dce28d959f2815b16f8179800000000000001022e58afe51f9ed8a\
            d3cc7897f634d881fdbe49a81564629ded8156bebd2ffd1af000000000000000000\
            000000000000000000000000000000000000000000007f00000126070000",
        )
        .unwrap();
        test_encoding_roundtrip(&open_channel, data).unwrap();
    }
}

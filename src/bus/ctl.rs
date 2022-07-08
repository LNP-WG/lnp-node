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

use amplify::num::u24;
use amplify::Slice32;
use bitcoin::Txid;
use internet2::addr::NodeAddr;
use internet2::presentation::sphinx::Hop;
use lnp::channel::bolt::{CommonParams, LocalKeyset, PeerParams, Policy};
use lnp::p2p::bolt::{ChannelId, OpenChannel, PaymentOnion};
use lnp::router::gossip::LocalChannelInfo;
use lnp_rpc::{ChannelInfo, Failure, PeerInfo};
use microservices::util::OptionDetails;
use strict_encoding::{NetworkDecode, NetworkEncode};
use wallet::hlc::HashLock;
use wallet::psbt::Psbt;
use wallet::scripts::PubkeyScript;

use crate::rpc::{ClientId, ServiceId};

/// RPC API requests over CTL message bus between LNP Node daemons and from/to clients.
#[derive(Clone, Debug, Display, From)]
#[derive(NetworkEncode, NetworkDecode)]
#[non_exhaustive]
pub enum CtlMsg {
    #[display("hello()")]
    Hello,

    // Node connectivity API
    // ---------------------
    // Sent from lnpd to peerd
    #[display("get_info()")]
    GetInfo,

    #[display("ping_peer()")]
    PingPeer,

    // Channel creation API
    // --------------------
    /// Initiates creation of a new channel by a local node. Sent from lnpd to a newly instantiated
    /// channeld.
    #[display("open_channel_with({0})")]
    OpenChannelWith(OpenChannelWith),

    /// Initiates acceptance of a new channel proposed by a remote node. Sent from lnpd to a newly
    /// instantiated channeld.
    #[display("accept_channel_from({0})")]
    AcceptChannelFrom(AcceptChannelFrom),

    /// Constructs funding PSBT to fund a locally-created new channel. Sent from peerd to lnpd.
    #[display("construct_funding({0})")]
    ConstructFunding(FundChannel),

    /// Provides channeld with the information about funding transaction output used to fund the
    /// newly created channel. Sent from lnpd to channeld.
    #[display("funding_constructed(...)")]
    FundingConstructed(Psbt),

    /// Signs previously prepared funding transaction and publishes it to bitcoin network. Sent
    /// from channeld to lnpd upon receival of `funding_signed` message from a remote peer.
    #[display("publish_funding({0})")]
    PublishFunding,

    // On-chain tracking API
    // ---------------------
    /// Asks on-chain tracking service to send updates on the transaction mining status
    #[display("track({txid}, {depth})")]
    Track { txid: Txid, depth: u32 },

    /// Asks on-chain tracking service to stop sending updates on the transaction mining status
    #[display("untrack({0})")]
    Untrack(Txid),

    /// Reports changes in the mining status for previously requested transaction tracked by an
    /// on-chain service
    #[display("tx_found({0})")]
    TxFound(TxStatus),

    // Routing & payments
    /// Request to channel daemon to perform payment using provided route
    #[display("payment(...)")]
    Payment { route: Vec<Hop<PaymentOnion>>, hash_lock: HashLock, enquirer: ClientId },

    /// Notifies routing daemon about a new local channel
    #[display("channel_created({0})")]
    ChannelCreated(LocalChannelInfo),

    /// Notifies routing daemon to remove information about a local channel
    #[display("channel_closed({0})")]
    ChannelClosed(ChannelId),

    /// Notifies routing daemon new balance of a local channel
    #[display("channel_balance_update({channel_id}, {local_amount_msat}+{remote_amount_msat})")]
    ChannelBalanceUpdate { channel_id: ChannelId, local_amount_msat: u64, remote_amount_msat: u64 },

    // Key-related tasks
    // -----------------
    #[display("sign(...)")]
    Sign(Psbt),

    #[display("signed(...)")]
    Signed(Psbt),

    // lnpd -> signd
    #[display("derive_keyset({0})")]
    DeriveKeyset(Slice32),

    // signd -> lnpd
    #[display("keyset({0}, ...)")]
    Keyset(ServiceId, LocalKeyset),

    // Responses
    // ---------
    #[display("progress(\"{0}\")")]
    #[from]
    Report(Report),

    /// Error returned back by response-reply type of daemons (like signed) in case if the
    /// operation has failed.
    #[display("error({destination}, \"{error}\")")]
    Error { destination: ServiceId, request: String, error: String },

    /// Error returned if the destination service is offline
    #[display("esb_error({destination}, \"{error}\")")]
    EsbError { destination: ServiceId, error: String },

    #[display("node_info({0})", alt = "{0:#}")]
    PeerInfo(PeerInfo),

    #[display("channel_info({0})", alt = "{0:#}")]
    ChannelInfo(ChannelInfo),
}

impl CtlMsg {
    pub fn with_error(
        destination: &ServiceId,
        message: &CtlMsg,
        err: &impl std::error::Error,
    ) -> CtlMsg {
        CtlMsg::Error {
            destination: destination.clone(),
            request: message.to_string(),
            error: err.to_string(),
        }
    }
}

/// Request configuring newly launched channeld instance
#[derive(Clone, PartialEq, Eq, Debug, Display, NetworkEncode, NetworkDecode)]
#[display("{remote_peer}, {funding_sat}, ...")]
pub struct OpenChannelWith {
    /// Node to open a channel with
    pub remote_peer: NodeAddr,

    /// Client identifier to report about the progress
    pub report_to: Option<ClientId>,

    /// Amount of satoshis for channel funding
    pub funding_sat: u64,

    /// Amount of millisatoshis to pay to the remote peer at the channel opening
    pub push_msat: u64,

    /// Channel policies
    pub policy: Policy,

    /// Channel common parameters
    pub common_params: CommonParams,

    /// Channel local parameters
    pub local_params: PeerParams,

    /// Channel local keyset
    pub local_keys: LocalKeyset,
}

/// Request configuring newly launched channeld instance
#[derive(Clone, PartialEq, Eq, Debug, Display, NetworkEncode, NetworkDecode)]
#[display("{remote_peer}, {channel_req}, ...")]
pub struct AcceptChannelFrom {
    /// Node to open a channel with
    pub remote_peer: NodeAddr,

    /// Client identifier to report about the progress
    pub report_to: Option<ServiceId>,

    /// Request received from a remote peer to open channel
    pub channel_req: OpenChannel,

    /// Channel policies
    pub policy: Policy,

    /// Channel common parameters
    pub common_params: CommonParams,

    /// Channel local parameters
    pub local_params: PeerParams,

    /// Channel local keyset
    pub local_keys: LocalKeyset,
}

/// Request information about constructing funding transaction
#[derive(Clone, PartialEq, Eq, Debug, Display, NetworkEncode, NetworkDecode)]
#[display("{script_pubkey}, {amount}")]
pub struct FundChannel {
    /// Address for the channel funding
    pub script_pubkey: PubkeyScript,

    /// Amount of funds to be sent to the funding address
    pub amount: u64,

    /// Fee rate to use for the funding transaction, per kilo-weight unit
    pub feerate_per_kw: Option<u32>,
}

/// Update on a transaction mining status
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Display)]
#[derive(NetworkEncode, NetworkDecode)]
#[display("{txid}, {depth}")]
pub struct TxStatus {
    /// Id of a transaction previously requested to be tracked
    pub txid: Txid,

    /// Depths from the chain tip
    pub depth: u24,

    /// Height of the block containing transaction
    pub height: u24,

    /// Transaction position within the block
    pub pos: u24,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Display, NetworkEncode, NetworkDecode)]
#[display("{client}, {status}")]
pub struct Report {
    pub client: ClientId,
    pub status: Status,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Display, From)]
#[derive(NetworkEncode, NetworkDecode)]
pub enum Status {
    #[display("progress = \"{0}\"")]
    #[from]
    Progress(String),

    #[display("success = {0}")]
    Success(OptionDetails),

    #[display("failure = {0}")]
    #[from]
    Failure(Failure),
}

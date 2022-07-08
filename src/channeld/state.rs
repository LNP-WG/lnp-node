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

use amplify::{DumbDefault, Slice32};
use bitcoin::hashes::Hash;
use internet2::addr::NodeId;
use lnp::channel::bolt::{BoltExt, CommonParams, LocalKeyset, PeerParams, Policy};
use lnp::p2p::bolt::TempChannelId;
use lnp::Channel;
use lnpbp::chain::Chain;

use super::automata::ChannelStateMachine;

/// State of the channel runtime which can persists and which evolution is automated with
/// different state machines.
#[derive(Default, StrictEncode, StrictDecode)]
pub(super) struct ChannelState {
    /// State machine managing the evolution of this state
    pub state_machine: ChannelStateMachine,

    /// Standard part of the channel state (defined in BOLTs)
    pub channel: Channel<BoltExt>,

    /// Runtime-specific (but persistable) part of the channel state: remote peer which is a
    /// counterparty of this channel.
    pub remote_id: Option<NodeId>,
}

impl ChannelState {
    pub fn with(temp_channel_id: TempChannelId, chain: &Chain) -> ChannelState {
        let chain_hash = chain.as_genesis_hash().into_inner();
        let channel = Channel::with(
            temp_channel_id,
            Slice32::from(chain_hash),
            Policy::default(),
            CommonParams::default(),
            PeerParams::default(),
            LocalKeyset::dumb_default(), // we do not have keyset derived at this stage
        );
        ChannelState { state_machine: Default::default(), channel, remote_id: None }
    }

    pub fn remote_id(&self) -> NodeId {
        self.remote_id.expect("remote peer must be present at this stage")
    }
}

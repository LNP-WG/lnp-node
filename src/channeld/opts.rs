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

use amplify::hex::FromHex;
use lnp::p2p::bolt::ChannelId;

use crate::peerd::KeyOpts;

/// Lightning peer network channel daemon; part of LNP Node.
///
/// The daemon is controlled though RPC socket (see `rpc-socket`).
#[derive(Parser, Clone, PartialEq, Eq, Debug)]
#[clap(name = "channeld", bin_name = "channeld", author, version)]
pub struct Opts {
    /// Node key configuration
    #[clap(flatten)]
    pub key_opts: KeyOpts,

    /// Channel id
    #[clap(parse(try_from_str = ChannelId::from_hex))]
    pub channel_id: ChannelId,

    /// Flag indicating that we are re-establishing a channel with the provided `channel_id`
    #[clap(short = 'R', long)]
    pub reestablish: bool,

    /// These params can be read also from the configuration file, not just
    /// command-line args or environment variables
    #[clap(flatten)]
    pub shared: crate::opts::Opts,
}

impl Opts {
    pub fn process(&mut self) {
        self.shared.process();
        self.key_opts.process(&self.shared);
    }
}

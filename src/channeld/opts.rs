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

use amplify::hex::FromHex;
use clap::ValueHint;
use internet2::PartialNodeAddr;
use lnp::p2p::legacy::ChannelId;

use crate::opts::FUNGIBLED_RPC_ENDPOINT;
use crate::peerd::KeyOpts;

/// Lightning peer network channel daemon; part of LNP Node
///
/// The daemon is controlled though ZMQ ctl socket (see `ctl-socket` argument
/// description)
#[derive(Parser, Clone, PartialEq, Eq, Debug)]
#[clap(name = "channeld", bin_name = "channeld", author, version)]
pub struct Opts {
    /// Node key configuration
    #[clap(flatten)]
    pub key_opts: KeyOpts,

    /// RGB configuration
    #[clap(flatten)]
    pub rgb_opts: RgbOpts,

    /// Channel id
    #[clap(parse(try_from_str = ChannelId::from_hex))]
    pub channel_id: ChannelId,

    /// These params can be read also from the configuration file, not just
    /// command-line args or environment variables
    #[clap(flatten)]
    pub shared: crate::opts::Opts,
}

/// RGB configuration
#[derive(Parser, Clone, PartialEq, Eq, Debug)]
pub struct RgbOpts {
    /// ZMQ socket name/address for RGB Node fungible RPC interface (RGB20 RPC)
    #[clap(
        short,
        long = "rgb20-rpc",
        global = true,
        env = "FUNGIBLED_RPC_ENDPOINT",
        value_hint = ValueHint::FilePath,
        default_value = &*FUNGIBLED_RPC_ENDPOINT
    )]
    pub rgb20_socket: PartialNodeAddr,
}

impl Opts {
    pub fn process(&mut self) {
        self.shared.process();
        self.key_opts.process(&self.shared);
    }
}

impl RgbOpts {
    pub fn process(&mut self, shared: &crate::opts::Opts) {
        match &mut self.rgb20_socket {
            PartialNodeAddr::ZmqIpc(path, ..) | PartialNodeAddr::Posix(path) => {
                shared.process_dir(path);
            }
            _ => {}
        }
    }
}

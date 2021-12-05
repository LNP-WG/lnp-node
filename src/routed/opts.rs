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

/// Lightning peer network routing daemon; part of LNP Node
///
/// The daemon is controlled though ZMQ ctl socket (see `ctl-socket` argument
/// description)
#[derive(Parser, Clone, PartialEq, Eq, Debug)]
#[clap(name = "routed", bin_name = "routed", author, version)]
pub struct Opts {
    /// RGB configuration: ignored by this daemon
    #[clap(short, long = "rgb20-rpc")]
    pub r: Option<String>,

    /// These params can be read also from the configuration file, not just
    /// command-line args or environment variables
    #[clap(flatten)]
    pub shared: crate::opts::Opts,
}

impl Opts {
    pub fn process(&mut self) {
        self.shared.process()
    }
}

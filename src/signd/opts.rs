// LNP Node: node running lightning network protocol and generalized lightning
// channels.
// Written in 2020-2024 by
//     Dr. Maxim Orlovsky <orlovsky@lnp-bp.org>
//
// To the extent possible under law, the author(s) have dedicated all
// copyright and related and neighboring rights to this software to
// the public domain worldwide. This software is distributed without
// any warranty.
//
// You should have received a copy of the MIT License along with this software.
// If not, see <https://opensource.org/licenses/MIT>.

use crate::opts::Options;

/// Lightning peer network channel daemon; part of LNP Node.
///
/// The daemon is controlled though RPC socket (see `rpc-socket`).
#[derive(Parser, Clone, PartialEq, Eq, Debug)]
#[clap(name = "signd", bin_name = "signd", author, version)]
pub struct Opts {
    /// These params can be read also from the configuration file, not just
    /// command-line args or environment variables
    #[clap(flatten)]
    pub shared: crate::opts::Opts,
}

impl Options for Opts {
    type Conf = ();

    fn shared(&self) -> &crate::opts::Opts { &self.shared }

    fn config(&self) -> Self::Conf { () }
}

impl Opts {
    pub fn process(&mut self) { self.shared.process() }
}

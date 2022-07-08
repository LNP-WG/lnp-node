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

use lnp::p2p;

use crate::opts::Options;
use crate::peerd::Opts;

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub struct Config {
    pub protocol: p2p::Protocol,
}

impl Options for Opts {
    type Conf = Config;

    fn shared(&self) -> &crate::opts::Opts { &self.shared }

    fn config(&self) -> Self::Conf { Config { protocol: self.protocol() } }
}

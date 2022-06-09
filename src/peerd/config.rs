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

use crate::opts::Options;
use crate::peerd::Opts;
use crate::P2pProtocol;

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub struct Config {
    pub protocol: P2pProtocol,
}

impl Options for Opts {
    type Conf = Config;

    fn shared(&self) -> &crate::opts::Opts { &self.shared }

    fn config(&self) -> Self::Conf {
        let protocol = if self.bolt {
            P2pProtocol::Bolt
        } else if self.bifrost {
            P2pProtocol::Bifrost
        } else {
            unreachable!()
        };
        Config { protocol }
    }
}

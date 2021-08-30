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

use internet2::NodeAddr;
use lnpbp::chain::Chain;

#[cfg(feature = "shell")]
use crate::opts::Opts;

/// Final configuration resulting from data contained in config file environment
/// variables and command-line options. For security reasons node key is kept
/// separately.
#[derive(Clone, PartialEq, Eq, Debug, Display)]
#[display(Debug)]
pub struct Config {
    /// Bitcoin blockchain to use (mainnet, testnet, signet, liquid etc)
    pub chain: Chain,

    /// ZMQ socket for lightning peer network message bus
    pub msg_endpoint: NodeAddr,

    /// ZMQ socket for internal service control bus
    pub ctl_endpoint: NodeAddr,
}

#[cfg(feature = "shell")]
impl From<Opts> for Config {
    fn from(opts: Opts) -> Self {
        Config {
            chain: opts.chain,
            msg_endpoint: opts.msg_socket.into(),
            ctl_endpoint: opts.ctl_socket.into(),
        }
    }
}

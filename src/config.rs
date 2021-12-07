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
use std::path::PathBuf;

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

    /// Directory for data files, like signing keys etc
    pub data_dir: PathBuf,

    /// ZMQ socket for lightning peer network message bus
    pub msg_endpoint: NodeAddr,

    /// ZMQ socket for internal service control bus
    pub ctl_endpoint: NodeAddr,

    /// URL for the electrum server connection
    pub electrum_url: String,
}

fn default_electrum_port(chain: &Chain) -> u16 {
    match chain {
        Chain::Mainnet => 50001,
        Chain::Testnet3 | Chain::Regtest(_) => 60001,
        Chain::Signet | Chain::SignetCustom(_) => 60601,
        Chain::LiquidV1 => 50501,
        Chain::Other(_) => 60001,
        _ => 60001,
    }
}

#[cfg(feature = "shell")]
impl From<Opts> for Config {
    fn from(opts: Opts) -> Self {
        let electrum_url = format!(
            "{}:{}",
            opts.electrum_server,
            opts.electrum_port
                .unwrap_or_else(|| default_electrum_port(&opts.chain))
        );

        Config {
            chain: opts.chain,
            data_dir: opts.data_dir,
            msg_endpoint: opts.msg_socket.into(),
            ctl_endpoint: opts.ctl_socket.into(),
            electrum_url,
        }
    }
}

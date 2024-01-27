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

#![allow(clippy::needless_borrow)] // due to a bug in `display(Debug)`

use std::fmt::Debug;
use std::path::PathBuf;

use internet2::addr::ServiceAddr;
use lnp::p2p::bolt::ActiveChannelId;
use lnpbp::chain::Chain;

use crate::opts::Options;

/// Final configuration resulting from data contained in config file environment
/// variables and command-line options. For security reasons node key is kept
/// separately.
#[derive(Clone, PartialEq, Eq, Debug, Display)]
#[display(Debug)]
pub struct Config<Ext = ()>
where
    Ext: Clone + Eq + Debug,
{
    /// Bitcoin blockchain to use (mainnet, testnet, signet, liquid etc)
    pub chain: Chain,

    /// Directory for data files, like signing keys etc
    pub data_dir: PathBuf,

    /// ZMQ socket for lightning peer network message bus
    pub msg_endpoint: ServiceAddr,

    /// ZMQ socket for internal service control bus
    pub ctl_endpoint: ServiceAddr,

    /// ZMQ socket for client-service RCP API.
    pub rpc_endpoint: ServiceAddr,

    /// URL for the electrum server connection
    pub electrum_url: String,

    /// Indicates whether deamons should be spawned as threads (true) or as child processes (false)
    pub threaded: bool,

    /// Daemon-specific config extensions
    pub ext: Ext,
}

// TODO: Move to descriptor wallet
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

impl<Ext> Config<Ext>
where
    Ext: Clone + Eq + Debug,
{
    pub fn with<Orig>(orig: Config<Orig>, ext: Ext) -> Self
    where
        Orig: Clone + Eq + Debug,
    {
        Config::<Ext> {
            chain: orig.chain,
            data_dir: orig.data_dir,
            msg_endpoint: orig.msg_endpoint,
            ctl_endpoint: orig.ctl_endpoint,
            rpc_endpoint: orig.rpc_endpoint,
            electrum_url: orig.electrum_url,
            threaded: orig.threaded,
            ext,
        }
    }

    pub fn channel_dir(&self) -> PathBuf {
        let mut channel_dir = self.data_dir.clone();
        channel_dir.push("channels");
        channel_dir
    }

    pub fn channel_file(&self, channel_id: ActiveChannelId) -> PathBuf {
        let mut channel_file = self.channel_dir();
        channel_file.push(channel_id.to_string());
        channel_file.set_extension("channel");
        channel_file
    }
}

#[cfg(feature = "server")]
impl<Opt> From<Opt> for Config<Opt::Conf>
where
    Opt: Options,
    Opt::Conf: Clone + Eq + Debug,
{
    fn from(opt: Opt) -> Self {
        let opts = opt.shared();

        let electrum_url = format!(
            "{}:{}",
            opts.electrum_server,
            opts.electrum_port.unwrap_or_else(|| default_electrum_port(&opts.chain))
        );

        let ctl_endpoint = match opts.threaded_daemons {
            true => ServiceAddr::Inproc(s!("lnp-ctl")),
            false => opts.ctl_endpoint.clone(),
        };

        Config {
            chain: opts.chain.clone(),
            data_dir: opts.data_dir.clone(),
            msg_endpoint: opts.msg_endpoint.clone(),
            ctl_endpoint,
            rpc_endpoint: opts.rpc_endpoint.clone(),
            electrum_url,
            threaded: opts.threaded_daemons,
            ext: opt.config(),
        }
    }
}

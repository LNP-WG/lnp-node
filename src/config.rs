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

#![allow(clippy::needless_borrow)] // due to a bug in `display(Debug)`

use std::fmt::Debug;
#[cfg(any(feature = "server", feature = "tor", feature = "embedded"))]
use std::net::SocketAddr;
use std::path::PathBuf;
#[cfg(any(feature = "server", feature = "tor", feature = "embedded"))]
use std::str::FromStr;

use internet2::addr::ServiceAddr;
use lnp::p2p::bolt::ActiveChannelId;
use lnpbp::chain::Chain;

use crate::opts::Options;
#[cfg(feature = "server")]
use crate::opts::{LNP_NODE_CTL_SOCKET, LNP_NODE_MSG_SOCKET};

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

    /// ZMQ socket for daemon RCP interface
    pub rpc_endpoint: ServiceAddr,

    /// URL for the electrum server connection
    pub electrum_url: String,

    /// Indicates whether deamons should be spawned as threads (true) or as child processes (false)
    pub threaded: bool,

    /// Daemon-specific config extensions
    pub ext: Ext,
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

        let (msg_default, ctl_default) = match opts.threaded_daemons {
            true => (s!("inproc://msg"), s!("inproc://ctl")),
            false => {
                let mut msg_default = LNP_NODE_MSG_SOCKET.to_owned();
                let mut ctl_default = LNP_NODE_CTL_SOCKET.to_owned();
                opts.process_dir(&mut msg_default);
                opts.process_dir(&mut ctl_default);
                (format!("ipc://{}", msg_default), format!("ipc://{}", ctl_default))
            }
        };

        let msg_endpoint = opts.msg_socket.as_ref().map(|s| match SocketAddr::from_str(&s) {
            Ok(_) => format!("tcp://{}", s),
            Err(_) => format!("ipc://{}", s),
        });

        let ctl_endpoint = opts.ctl_socket.as_ref().map(|s| match SocketAddr::from_str(&s) {
            Ok(_) => format!("tcp://{}", s),
            Err(_) => format!("ipc://{}", s),
        });

        let rpc_endpoint = match SocketAddr::from_str(&opts.rpc_socket) {
            Ok(_) => format!("tcp://{}", opts.rpc_socket),
            Err(_) => format!("ipc://{}", opts.rpc_socket),
        };

        Config {
            chain: opts.chain.clone(),
            data_dir: opts.data_dir.clone(),
            msg_endpoint: msg_endpoint
                .unwrap_or(msg_default)
                .parse()
                .expect("ZMQ sockets should be either TCP addresses or files"),
            ctl_endpoint: ctl_endpoint
                .unwrap_or(ctl_default)
                .parse()
                .expect("ZMQ sockets should be either TCP addresses or files"),
            rpc_endpoint: rpc_endpoint
                .parse()
                .expect("ZMQ sockets should be either TCP addresses or files"),
            electrum_url,
            threaded: opts.threaded_daemons,
            ext: opt.config(),
        }
    }
}

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

use clap::{AppSettings, Clap};

use lnpbp::lnp::NodeAddr;

/// Command-line tool for working with LNP node
#[derive(Clap, Clone, PartialEq, Eq, Debug)]
#[clap(
    name = "lnp-cli",
    bin_name = "lnp-cli",
    author,
    version,
    setting = AppSettings::ColoredHelp
)]
pub struct Opts {
    /// These params can be read also from the configuration file, not just
    /// command-line args or environment variables
    #[clap(flatten)]
    pub shared: crate::opts::Opts,

    /// Command to execute
    #[clap(subcommand)]
    pub command: Command,
}

impl Opts {
    pub fn process(&mut self) {
        self.shared.process()
    }
}

/// Command-line commands:
#[derive(Clap, Clone, PartialEq, Eq, Debug, Display)]
#[display(doc_comments)]
pub enum Command {
    /// Ping remote peer
    Ping,

    /// Establishes new channel
    CreateChannel {
        /// Address of the remote node, in
        /// '<public_key>@<ipv4>|<ipv6>|<onionv2>|<onionv3>[:<port>]' format
        #[clap()]
        node_addr: NodeAddr,
    },
}

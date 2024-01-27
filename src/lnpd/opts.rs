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

use lnp_rpc::ListenAddr;

use crate::opts::Options;
use crate::peerd::KeyOpts;

/// Lightning node management daemon; part of LNP Node.
///
/// The daemon is controlled though RPC socket (see `rpc-socket`).
#[derive(Parser, Clone, PartialEq, Eq, Debug)]
#[clap(name = "lnpd", bin_name = "lnpd", author, version)]
pub struct Opts {
    /// Node key configuration
    #[clap(flatten)]
    pub key_opts: KeyOpts,

    /// These params can be read also from the configuration file, not just
    /// command-line args or environment variables
    #[clap(flatten)]
    pub shared: crate::opts::Opts,

    /// Start daemon in listening mode binding the provided local address(es).
    ///
    /// Binds to the specified interface and listens for incoming connections, spawning
    /// a new thread / forking child process for each new incoming client connecting the
    /// opened socket. Whether the child is spawned as a thread or forked as a child
    /// process determined by the presence of `--threaded` flag.
    #[clap(short = 'L', long)]
    pub listen: Option<Vec<ListenAddr>>,

    /// If the argument is provided, the node binds to all network addresses and requires
    /// `--bifrost` and/or `--bolt` arguments.
    #[clap(long, conflicts_with = "listen")]
    pub listen_all: bool,

    /// Use BOLT protocol for listening for the incoming connections. Can optionally specify a
    /// custom port number.
    #[clap(long, conflicts_with = "listen", requires = "listen-all")]
    pub bolt: Option<Option<u16>>,

    /// Use Bifrost protocol for listening for the incoming connections. Can optionally specify a
    /// custom port number.
    #[clap(long, conflicts_with = "listen", requires = "listen-all")]
    pub bifrost: Option<Option<u16>>,

    /// Optional command to execute and exit
    #[clap(subcommand)]
    pub command: Option<Command>,
}

impl Options for Opts {
    type Conf = ();

    fn shared(&self) -> &crate::opts::Opts { &self.shared }

    fn config(&self) -> Self::Conf { () }
}

#[derive(Subcommand, Clone, PartialEq, Eq, Debug)]
pub enum Command {
    /// Initialize data directory
    Init,
}

impl Opts {
    pub fn process(&mut self) {
        self.shared.process();
        self.key_opts.process(&self.shared);
    }
}

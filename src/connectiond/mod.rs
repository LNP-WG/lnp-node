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

#[cfg(feature = "shell")]
mod opts;
mod peer;
mod rpc;
mod runtime;

#[cfg(feature = "shell")]
pub use opts::Opts;
pub use runtime::run;
pub(self) use runtime::Runtime;

use lnpbp::lnp::NodeEndpoint;

/// Final configuration resulting from data contained in config file environment
/// variables and command-line options. For security reasons node key is kept
/// separately.
#[derive(Clone, PartialEq, Eq, Debug, Display)]
#[display(Debug)]
pub struct Config {
    /// ZMQ RPC socket for transmitting lightning peer network messages
    pub msg_endpoint: NodeEndpoint,

    /// ZMQ RPC socket for internal daemon control bus
    pub ctl_endpoint: NodeEndpoint,
}

#[cfg(feature = "shell")]
impl From<Opts> for Config {
    fn from(opts: Opts) -> Self {
        Config {
            msg_endpoint: opts.shared.msg_socket.into(),
            ctl_endpoint: opts.shared.ctl_socket.into(),
        }
    }
}

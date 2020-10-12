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

#[cfg(feature = "daemon")]
pub const LNP_CONFIG: &'static str = "{data_dir}/lnpd.toml";
#[cfg(feature = "cli")]
pub const LNP_CLI_CONFIG: &'static str = "{data_dir}/lnp-cli.toml";
pub const LNP_DATA_DIR: &'static str = "/var/lib/lnp";
pub const LNP_ZMQ_ENDPOINT: &'static str = "tcp://0.0.0.0:20202"; //"ipc:{data_dir}/zmq.rpc";
#[cfg(feature = "daemon")]
pub const LNP_TCP_ENDPOINT: &'static str = "0.0.0.0:20202";

pub use lnpbp::bitcoin::secp256k1::{self, Secp256k1};

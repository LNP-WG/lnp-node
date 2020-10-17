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

pub const LNP_NODE_CONFIG: &'static str = "{data_dir}/lnpd.toml";
pub const LNP_NODE_DATA_DIR: &'static str = "/var/lib/lnp";

pub const LNP_NODE_MSG_SOCKET_NAME: &'static str = "msg.rpc";
pub const LNP_NODE_CTL_SOCKET_NAME: &'static str = "ctl.rpc";

pub const LNP_NODE_BIND: &'static str = "0.0.0.0:20202";
pub const LNP_NODE_TOR_PROXY: &'static str = "127.0.0.1:9050";

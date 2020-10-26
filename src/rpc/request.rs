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

use lnpbp::lnp::application::Messages;
use lnpbp::lnp::rpc_connection;

#[derive(Clone, Debug, Display, LnpApi)]
#[lnp_api(encoding = "strict")]
#[display(Debug)]
#[non_exhaustive]
pub enum Request {
    #[lnp_api(type = 1)]
    LnpwpMessage(Messages),

    #[lnp_api(type = 2)]
    InitConnection,

    #[lnp_api(type = 3)]
    PingPeer,
}

impl rpc_connection::Request for Request {}

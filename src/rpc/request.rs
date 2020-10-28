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

use lnpbp::lnp::rpc_connection;
use lnpbp::lnp::{message, Messages};

use crate::DaemonId;

#[derive(Clone, Debug, Display, LnpApi)]
#[lnp_api(encoding = "strict")]
#[non_exhaustive]
pub enum Request {
    #[lnp_api(type = 0)]
    #[display("hello()")]
    Hello,

    #[lnp_api(type = 1)]
    #[display("lnpwp({_0})")]
    LnpwpMessage(Messages),

    #[lnp_api(type = 2)]
    #[display("ping_peer()")]
    PingPeer,

    #[lnp_api(type = 3)]
    #[display("create_channel(...)")]
    CreateChannel(CreateChannel),
}

impl rpc_connection::Request for Request {}

#[derive(Clone, PartialEq, Eq, Debug, Display, StrictEncode, StrictDecode)]
#[display(Debug)]
pub struct CreateChannel {
    pub channel_req: message::OpenChannel,
    pub connectiond: DaemonId,
}

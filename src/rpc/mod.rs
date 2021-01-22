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

mod client;
mod reply;
pub mod request;

pub use client::Client;
pub use reply::Reply;
pub use request::Request;

use microservices::esb::BusId;
use microservices::rpc_connection::Api;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Display)]
pub enum ServiceBus {
    #[display("MSG")]
    Msg,
    #[display("CTL")]
    Ctl,
    #[display("BRIDGE")]
    Bridge,
}

impl BusId for ServiceBus {}

pub struct Rpc {}

impl Api for Rpc {
    type Request = Request;
    type Reply = Reply;
}

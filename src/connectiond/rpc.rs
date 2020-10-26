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

use lnpbp_services::rpc::{Failure, Handler};

use super::Runtime;
use crate::rpc::{Endpoints, Reply, Request, Rpc};
use crate::Error;

impl Handler<Endpoints> for Runtime {
    type Api = Rpc;
    type Error = Error;

    fn handle(
        &mut self,
        endpoint: Endpoints,
        request: Request,
    ) -> Result<Reply, Self::Error> {
        unimplemented!()
    }
}

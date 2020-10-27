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

use lnpbp_services::shell::Exec;

use super::{Command, Runtime};
use crate::rpc::Request;
use crate::Error;

impl Exec for Command {
    type Runtime = Runtime;
    type Error = Error;

    fn exec(&self, runtime: &mut Self::Runtime) -> Result<(), Self::Error> {
        debug!("Performing {:?}: {}", self, self);
        let info = match self {
            Command::Init => runtime.request(Request::InitConnection)?,
            Command::Ping => runtime.request(Request::PingPeer)?,
        };
        info!("{}", info);
        Ok(())
    }
}

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

use lnpbp::lnp::transport::zmq::SocketLocator;

use crate::constants::LNP_ZMQ_ENDPOINT;

#[derive(Clone, PartialEq, Eq, Debug, Display)]
#[display(Debug)]
pub struct Config {
    pub endpoint: SocketLocator,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            endpoint: LNP_ZMQ_ENDPOINT
                .parse()
                .expect("Error in LNP_ZMQ_ENDPOINT constant value"),
        }
    }
}

// Lightning network protocol (LNP) daemon suite
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


use std::convert::TryFrom;
use lnpbp::lnp::NodeAddr;
use lnpbp::internet::InetSocketAddr;

use super::*;


#[derive(Clone, Copy, Debug, Display)]
#[display_from(Debug)]
pub struct Connect {
    pub node_addr: NodeAddr,
}

impl Procedure<'_> for Connect { }

impl TryFrom<&[zmq::Message]> for Connect {
    type Error = Error;

    fn try_from(args: &[zmq::Message]) -> Result<Self, Self::Error> {
        if args.len() != 2 { Err(Error::WrongNumberOfArguments)? }

        let node_id = secp256k1::PublicKey::from_slice(&args[0][..])?;
        let inet_addr = InetSocketAddr::from_uniform_encoding(&args[1])
            .ok_or(Error::MalformedArgument)?;

        Ok(Self {
            node_addr: NodeAddr { node_id, inet_addr }
        })
    }
}

impl From<Connect> for Multipart {
    fn from(proc: Connect) -> Self {
        vec![
            zmq::Message::from(&proc.node_addr.node_id.serialize()[..]),
            zmq::Message::from(&proc.node_addr.inet_addr.to_uniform_encoding()[..])
        ]
    }
}

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


use std::convert::{TryFrom, TryInto};

use lnpbp::lightning::bitcoin;
use lnpbp::lightning::bitcoin::secp256k1;
use lnpbp::lnp::NodeAddr;
use lnpbp::internet::InetSocketAddr;


type Multipart = Vec<zmq::Message>;

const MSGID_CONNECT: u16 = 0x0001;


#[derive(Debug, Display)]
#[display_from(Debug)]
pub enum Error {
    MessageBusError(zmq::Error),
    MalformedRequest,
    MalformedCommand,
    MalformedArgument,
    UnknownCommand,
    WrongNumberOfArguments
}

impl std::error::Error for Error {}

impl From<bitcoin::consensus::encode::Error> for Error {
    fn from(_: bitcoin::consensus::encode::Error) -> Self {
        Error::MalformedArgument
    }
}

impl From<secp256k1::Error> for Error {
    fn from(_: secp256k1::Error) -> Self {
        Error::MalformedArgument
    }
}

pub trait Procedure { }

pub struct Connect {
    pub node_addr: NodeAddr,
}

impl Procedure for Connect { }

impl TryFrom<&[zmq::Message]> for Connect {
    type Error = Error;

    fn try_from(args: &[zmq::Message]) -> Result<Self, Self::Error> {
        if args.len() != 3 { Err(Error::WrongNumberOfArguments)? }

        let pk = secp256k1::PublicKey::from_slice(&args[0][..])?;
        let socket_addr = InetSocketAddr::from_uniform_encoding(&args[1])
            .ok_or(Error::MalformedArgument)?;

        Ok(Self {
            node_addr: NodeAddr { id: pk, socket_address: socket_addr }
        })
    }
}

impl From<Connect> for Multipart {
    fn from(proc: Connect) -> Self {
        vec![
            zmq::Message::from(&MSGID_CONNECT.to_be_bytes()[..]),
            zmq::Message::from(&proc.node_addr.id.serialize()[..]),
            zmq::Message::from(&proc.node_addr.socket_address.to_uniform_encoding()[..])
        ]
    }
}

pub enum Command {
    Connect(Connect)
}

impl TryFrom<Multipart> for Command {
    type Error = Error;

    fn try_from(multipart: Multipart) -> Result<Self, Self::Error> {
        let (cmd, args) = multipart.split_first()
            .ok_or(Error::MalformedRequest)
            .and_then(|(cmd_data, args)| {
                if cmd_data.len() != 2 {
                    Err(Error::MalformedCommand)?
                }
                let mut buf = [0u8; 2];
                buf.clone_from_slice(&cmd_data[0..2]);
                Ok((u16::from_be_bytes(buf), args))
            })?;

        Ok(match cmd {
            MSGID_CONNECT => Command::Connect(args.try_into()?),
            _ => Err(Error::UnknownCommand)?,
        })
    }
}

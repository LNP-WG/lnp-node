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


use lnpbp::lightning::bitcoin;
use lnpbp::lightning::bitcoin::secp256k1;
use lnpbp::lnp;


#[derive(Debug, Display)]
#[display_from(Debug)]
pub enum Error {
    MessageBusError(zmq::Error),
    ConnectionError(lnp::ConnectionError),
    MalformedRequest,
    MalformedCommand,
    MalformedArgument,
    UnknownCommand,
    WrongNumberOfArguments
}

impl std::error::Error for Error {}

impl From<Error> for String {
    fn from(err: Error) -> Self { format!("{}", err) }
}

impl From<zmq::Error> for Error {
    fn from(err: zmq::Error) -> Self {
        Error::MessageBusError(err)
    }
}

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

impl From<lnp::ConnectionError> for Error {
    fn from(err: lnp::ConnectionError) -> Self {
        Error::ConnectionError(err)
    }
}

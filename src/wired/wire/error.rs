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


use std::io;
use crate::wired::BootstrapError;

#[derive(Debug, Display)]
#[display_from(Debug)]
pub enum Error {
    IpSocketError(io::Error),
    PeerBootsrapError(BootstrapError)
}

impl std::error::Error for Error {}


impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::IpSocketError(err)
    }
}

impl From<BootstrapError> for Error {
    fn from(err: BootstrapError) -> Self {
        Error::PeerBootsrapError(err)
    }
}

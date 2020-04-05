// Bitcoin transaction processing & database indexing daemon
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


use std::error::Error;
use tokio::task::JoinError;


#[derive(Debug, Display)]
#[display_from(Debug)]
pub enum BootstrapError {
    MultithreadError(JoinError)
}

impl From<JoinError> for BootstrapError {
    fn from(err: JoinError) -> Self {
        BootstrapError::MultithreadError(err)
    }
}

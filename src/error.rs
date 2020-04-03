// Lightning network protocol (LNP) daemon
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
use diesel::{
    ConnectionError,
    result::Error as DieselError,
};

#[derive(Debug, Display)]
#[display_from(Debug)]
pub enum BootstrapError {
    IPCSocketError(zmq::Error, IPCSocket, Option<String>),
    APISocketError(zmq::Error, APISocket, Option<String>),
    MonitorSocketError(Box<dyn Error>),
    StateDBConnectionError(ConnectionError),
    StateDBIntegrityError,
    StateDBError(DieselError),
    MultithreadError(JoinError)
}

impl From<JoinError> for BootstrapError {
    fn from(err: JoinError) -> Self {
        BootstrapError::MultithreadError(err)
    }
}

impl From<ConnectionError> for BootstrapError {
    fn from(err: ConnectionError) -> Self {
        BootstrapError::StateDBConnectionError(err)
    }
}

impl From<DieselError> for BootstrapError {
    fn from(err: DieselError) -> Self {
        BootstrapError::StateDBError(err)
    }
}

#[derive(Debug, Display)]
#[display_from(Debug)]
pub enum IPCSocket {
}

#[derive(Debug, Display)]
#[display_from(Debug)]
pub enum APISocket {
    PubSub,
    ReqRep,
}

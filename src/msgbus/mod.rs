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


pub mod constants;
mod error;
mod command;
pub mod proc;

pub use error::*;
pub use command::*;
pub use proc::*;


use std::convert::{TryFrom, TryInto};

use lnpbp::lightning::bitcoin;
use lnpbp::lightning::bitcoin::secp256k1;
use lnpbp::lnp::NodeAddr;
use lnpbp::internet::InetSocketAddr;


pub type Multipart = Vec<zmq::Message>;


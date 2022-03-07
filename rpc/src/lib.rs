// LNP Node: node running lightning network protocol and generalized lightning
// channels.
// Written in 2020-2022 by
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

#![recursion_limit = "256"]
// Coding conventions
#![allow(clippy::large_enum_variant)]
#![deny(
    non_upper_case_globals,
    non_camel_case_types,
    non_snake_case,
    unused_mut,
    unused_imports,
    dead_code
    // missing_docs,
)]

#[macro_use]
extern crate amplify;
#[macro_use]
extern crate strict_encoding;
#[macro_use]
extern crate internet2;
#[macro_use]
extern crate log;

#[cfg(feature = "serde")]
extern crate serde_crate as serde;
#[cfg(feature = "serde")]
#[macro_use]
extern crate serde_with;

mod client;
mod error;
mod messages;
mod service_id;

pub use client::Client;
pub use error::Error;
pub use messages::*;
pub use service_id::{ClientId, ClientName, ServiceId};

pub const LNP_NODE_RPC_SOCKET: &str = "127.0.0.1:62962";

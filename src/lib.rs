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

#![recursion_limit = "256"]
// Coding conventions
#![deny(
    non_upper_case_globals,
    non_camel_case_types,
    non_snake_case,
    unused_mut,
    // unused_imports,
    // dead_code
    // missing_docs,
)]

#[macro_use]
extern crate amplify;
#[macro_use]
extern crate strict_encoding;
#[cfg_attr(feature = "_rpc", macro_use)]
extern crate internet2;

#[cfg(feature = "shell")]
#[macro_use]
extern crate clap;
#[macro_use]
extern crate log;

#[cfg(feature = "serde")]
extern crate serde_crate as serde;
#[cfg(feature = "serde")]
#[macro_use]
extern crate serde_with;

#[cfg(feature = "cli")]
pub mod cli;
#[cfg(feature = "_rpc")]
mod config;
mod error;
#[cfg(feature = "_rpc")]
pub mod i9n;
#[cfg(feature = "shell")]
pub mod opts;
#[cfg(feature = "_rpc")]
pub mod state_machine;

#[cfg(feature = "node")]
pub mod channeld;
#[cfg(feature = "node")]
pub mod gossipd;
#[cfg(feature = "node")]
pub mod lnpd;
#[cfg(feature = "node")]
pub mod peerd;
#[cfg(feature = "node")]
pub mod routed;
#[cfg(feature = "_rpc")]
mod service;
#[cfg(feature = "node")]
pub mod signd;

#[cfg(feature = "_rpc")]
pub use config::Config;
pub use error::Error;
#[cfg(feature = "_rpc")]
pub use service::{CtlServer, Endpoints, LogStyle, Service, ServiceId, TryToServiceId};

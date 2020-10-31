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
extern crate amplify_derive;
#[macro_use]
extern crate lnpbp_derive;

#[cfg(feature = "shell")]
extern crate clap;
#[cfg(feature = "shell")]
#[macro_use]
extern crate log;

#[cfg(feature = "cli")]
pub mod cli;
#[cfg(any(feature = "node", feature = "client"))]
mod config;
mod error;
#[cfg(feature = "shell")]
pub mod opts;
#[cfg(any(feature = "node", feature = "client"))]
pub mod rpc;

#[cfg(feature = "node")]
pub mod channeld;
#[cfg(feature = "node")]
pub mod connectiond;
#[cfg(feature = "node")]
pub mod gossipd;
#[cfg(feature = "node")]
pub mod lnpd;
#[cfg(feature = "node")]
pub mod routed;
#[cfg(any(feature = "node", feature = "client"))]
mod service;

#[cfg(any(feature = "node", feature = "client"))]
pub use config::Config;
pub use error::Error;
#[cfg(any(feature = "node", feature = "client"))]
pub use service::{ClientName, LogStyle, Service, ServiceId};

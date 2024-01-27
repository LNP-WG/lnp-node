// LNP Node: node running lightning network protocol and generalized lightning
// channels.
// Written in 2020-2024 by
//     Dr. Maxim Orlovsky <orlovsky@lnp-bp.org>
//
// To the extent possible under law, the author(s) have dedicated all
// copyright and related and neighboring rights to this software to
// the public domain worldwide. This software is distributed without
// any warranty.
//
// You should have received a copy of the MIT License along with this software.
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
    // dead_code
    // missing_docs,
)]
#![allow(dead_code)]

#[macro_use]
extern crate amplify;
#[macro_use]
extern crate internet2;
#[macro_use]
extern crate strict_encoding;

#[cfg(feature = "server")]
#[macro_use]
extern crate clap;
#[macro_use]
extern crate log;

pub use lnp_rpc as rpc;

pub mod automata;
pub mod bus;
mod config;
mod error;
#[cfg(feature = "server")]
pub mod opts;

pub mod channeld;
pub mod lnpd;
pub mod peerd;
pub mod routed;
mod service;
pub mod signd;
pub mod watchd;

pub use config::Config;
pub use error::Error;
pub use service::{BridgeHandler, Endpoints, Responder, Service, TryToServiceId};

pub const LNP_NODE_MASTER_KEY_FILE: &str = "master.key";
pub const LNP_NODE_FUNDING_WALLET: &str = "funding.wallet";

#[cfg(not(any(feature = "bolt", feature = "bifrost")))]
compile_error!("either 'bolt' or 'bifrost' feature must be used");

// TODO: React on reestablish message
// TODO: Lnpd must store channel launcher state
// TODO: Channel daemon must store its own state to data directory and re-load it
// TODO: Make onchaind

// Refactoring todos:
// TODO: Refactor client reporting

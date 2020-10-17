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

#![feature(never_type)]
#![recursion_limit = "256"]
// Coding conventions
#![deny(
    non_upper_case_globals,
    non_camel_case_types,
    non_snake_case,
    unused_mut,
    unused_imports,
    dead_code
)]
// TODO: when we will be ready for the release #![deny(missing_docs)]
// #![warn(missing_docs)]

/*
#[macro_use]
extern crate amplify;
#[macro_use]
extern crate amplify_derive;
#[macro_use]
pub extern crate lnpbp;
#[macro_use]
pub extern crate lnpbp_derive;

#[macro_use]
extern crate clap;
#[macro_use]
extern crate log;

#[macro_use]
pub extern crate serde_with;
*/

pub mod connectiond;
pub mod opts;

/*
pub mod cli;
//pub mod i9n;
pub mod rpc;
*/

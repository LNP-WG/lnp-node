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
    unused_imports,
    dead_code
    // missing_docs,
)]

//! Command-line interface to LNP node

#[macro_use]
extern crate amplify;
#[macro_use]
extern crate log;
#[macro_use]
extern crate clap;

mod command;
mod opts;

use clap::Parser;
use lnp_node::rpc::Client;
use lnp_node::LogStyle;
use microservices::shell::{Exec, LogLevel};

pub use crate::opts::{Command, Opts};

fn main() {
    println!("lnp-cli: command-line tool for working with LNP node");

    let opts = Opts::parse();
    LogLevel::from_verbosity_flag_count(opts.verbose).apply();

    trace!("Command-line arguments: {:?}", opts);

    let mut client = Client::with(&opts.connect).expect("Error initializing client");

    trace!("Executing command: {:?}", opts.command);
    opts.command.exec(&mut client).unwrap_or_else(|err| eprintln!("{}", err.err()));
}

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

//! Command-line interface to LNP Node

#[macro_use]
extern crate amplify;
#[macro_use]
extern crate log;
#[macro_use]
extern crate clap;

mod command;
mod opts;

#[cfg(not(any(feature = "bolt", feature = "bifrost")))]
compile_error!("either 'bolt' or 'bifrost' feature must be used");

use clap::Parser;
use internet2::addr::ServiceAddr;
use lnp_rpc::Client;
use microservices::cli::LogStyle;
use microservices::shell::{Exec, LogLevel};

pub use crate::opts::{Command, Opts};

fn main() {
    println!("lnp-cli: command-line tool for working with LNP node");

    let opts = Opts::parse();
    LogLevel::from_verbosity_flag_count(opts.verbose).apply();

    let mut connect = opts.connect.clone();
    if let ServiceAddr::Ipc(ref mut path) = connect {
        *path = shellexpand::tilde(path).to_string();
    }
    debug!("RPC socket {}", connect);
    trace!("Command-line arguments: {:?}", opts);

    let mut client = Client::with(connect).expect("Error initializing client");

    trace!("Executing command: {:?}", opts.command);
    opts.exec(&mut client)
        .unwrap_or_else(|err| eprintln!("{} {}\n", "Error:".err(), err.err_details()));
}

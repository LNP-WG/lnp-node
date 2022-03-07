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
#![deny(
    non_upper_case_globals,
    non_camel_case_types,
    non_snake_case,
    unused_mut,
    unused_imports,
    dead_code,
    missing_docs
)]

//! Main executable for signd: lightning peer network microservice signing
//! transactions.

#[macro_use]
extern crate log;

use clap::Parser;
use lnp_node::signd::{self, Opts};
use lnp_node::Config;

fn main() {
    println!("signd: lightning peer network gossip daemon");

    let mut opts = Opts::parse();
    trace!("Command-line arguments: {:?}", &opts);
    opts.process();
    trace!("Processed arguments: {:?}", &opts);

    let config: Config = opts.shared.clone().into();
    trace!("Daemon configuration: {:?}", &config);
    debug!("MSG RPC socket {}", &config.msg_endpoint);
    debug!("CTL RPC socket {}", &config.ctl_endpoint);

    /*
    use self::internal::ResultExt;
    let (config_from_file, _) =
        internal::Config::custom_args_and_optional_files(std::iter::empty::<
            &str,
        >())
        .unwrap_or_exit();
     */

    debug!("Starting runtime ...");
    signd::run(config).expect("Error running signd runtime");

    unreachable!()
}

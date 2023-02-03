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
//! Main executable for swapd: microservice for performing submarine swap.
//! transactions.

#[macro_use]
extern crate log;

use clap::Parser;
use lnp_node::swapd::Opts;
use lnp_node::Config;

fn main() {
    println!("swapd: submarine swap service");

    let mut opts = Opts::parse();
    trace!("Command-line arguments: {:?}", &opts);
    opts.process();
    trace!("Processed arguments: {:?}", &opts);

    let config: Config = opts.clone().into();
    trace!("Daemon configuration: {:?}", &config);
    debug!("MSG RPC socket {}", &config.msg_endpoint);
    debug!("CTL RPC socket {}", &config.ctl_endpoint);

    debug!("Starting runtime ...");
    // swapd::run(config).expect("Error running swapd runtime");

    todo!();
    unreachable!()
}

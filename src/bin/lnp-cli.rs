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

//! Command-line interface to LNP node

#![feature(never_type)]

#[macro_use]
extern crate log;

use amplify::Exec;
use clap::Clap;
use log::LevelFilter;
use std::convert::TryInto;

use lnp_node::cli::{Config, Opts, Runtime};
use lnp_node::error::BootstrapError;

#[tokio::main]
async fn main() -> Result<(), BootstrapError> {
    log::set_max_level(LevelFilter::Trace);
    debug!("Command-line interface to LNP node");

    let opts: Opts = Opts::parse();
    let config: Config = opts.clone().try_into()?;
    config.apply();

    let mut runtime = Runtime::init(config).await?;

    trace!("Executing command: {:?}", opts.command);
    opts.command
        .exec(&mut runtime)
        .unwrap_or_else(|err| error!("{}", err));

    Ok(())
}

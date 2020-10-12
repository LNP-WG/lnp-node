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

//! Main executable for wired: lightning network protocol peer wire service

#![feature(never_type)]

#[macro_use]
extern crate log;

use amplify::TryService;
use clap::Clap;
use core::convert::TryInto;
use log::LevelFilter;

use lnp_node::error::BootstrapError;
use lnp_node::wired::{Config, Opts, Runtime};

#[tokio::main]
async fn main() -> Result<!, BootstrapError> {
    log::set_max_level(LevelFilter::Trace);
    info!("wired: lightning network protocol peer wire service");

    let opts: Opts = Opts::parse();
    let config: Config = opts.clone().try_into()?;
    config.apply();

    let runtime = Runtime::init(config).await?;
    runtime.run_or_panic("wired runtime").await;

    unreachable!()
}

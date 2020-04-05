// Lightning network protocol (LNP) daemon
// Lightning network protocol (LNP) daemon
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


// We need this since code is not completed and a lot of it is written
// for future functionality
// Remove this once the first version will be complete
#![allow(dead_code)]
#![allow(unused_variables)]
// In mutithread environments it's critical to capture all failures
#![deny(unused_must_use)]

#![feature(never_type)]
#![feature(unwrap_infallible)]
#![feature(in_band_lifetimes)]

extern crate tokio;
extern crate futures;
extern crate zmq;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate clap;
#[macro_use]
extern crate derive_wrapper;
#[macro_use]
extern crate async_trait;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate dotenv;
extern crate chrono;
extern crate tiny_http;
extern crate prometheus;
extern crate lightning;
extern crate lnpbp;

mod error;
// TODO: Uncomment after diesel setup
//mod schema;
mod monitor;
mod constants;
mod service;
mod config;
mod peer;

mod api;

use std::env;
use log::*;
use futures::future;
use tokio::task::JoinHandle;
use crate::{
    error::*,
    service::*,
    config::Config,
    constants::*,
};

#[tokio::main]
async fn main() -> Result<(), BootstrapError> {
    println!("\nlnpd: Lightning network protocol daemon\n");

    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "trace");
    }
    env_logger::init();
    log::set_max_level(LevelFilter::Trace);

    // TODO: Init config from command-line arguments, environment and config file

    let config = Config::default();

    let mut context = zmq::Context::new();

    let monitor_task = monitor::run(config.clone().into(), &mut context)?;

    let api_task = api::run(config.clone().into(), &mut context)?;

    let tasks: Vec<JoinHandle<!>> = vec![
        monitor_task,
        api_task
    ].into_iter().flatten().collect();
    future::try_join_all(tasks).await?;

    Ok(())
}

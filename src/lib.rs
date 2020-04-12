// Lightning network protocol (LNP) daemon suit
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

#[macro_use]
extern crate tokio;
extern crate futures;
extern crate zmq;
extern crate diesel;
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
extern crate lnpbp;
#[macro_use]
extern crate serde_derive;
extern crate configure_me;

pub mod msgbus;
pub mod service;
pub mod wired;
pub mod error;

pub use service::*;
pub use error::*;

pub mod cli;

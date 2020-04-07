// Lightning network protocol (LNP) daemon suite
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

use std::env;
use std::sync::Arc;
use std::str::FromStr;
use log::*;
use clap::Clap;
#[macro_use]
use tokio::try_join;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio::net::TcpStream;
#[macro_use]
use lnpbp::common::internet::{InetAddr, InetSocketAddr};

use lnpd::service::*;
use lnpd::wired::*;

#[tokio::main]
async fn main() -> Result<(), BootstrapError> {
    // TODO: Parse config file as well
    let opts: Opts = Opts::parse();
    let config: Config = opts.into();

    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", match config.verbose {
            0 => "error",
            1 => "warn",
            2 => "info",
            3 => "debug",
            4 => "trace",
            _ => "trace",
        });
    }
    env_logger::init();
    log::set_max_level(LevelFilter::Trace);

    let mut context = zmq::Context::new();

    let wire_sockets = Arc::new(Mutex::new(Vec::<Arc<TcpStream>>::new()));
    let wire_threads = Arc::new(Mutex::new(Vec::<JoinHandle<!>>::new()));

    let wire_service = WireService::init(
        config.clone().into(),
        config.clone().into(),
        context.clone(),
        wire_sockets.clone(),
        wire_threads.clone()
    ).await?;
    let bus_service = BusService::init(
        config.clone().into(),
        context.clone(),
        wire_sockets.clone()
    )?;

    let wire_addr = config.lnp2p_addr.clone();
    let bus_addr = config.msgbus_peer_api_addr.clone();

    try_join!(
        tokio::spawn(async move {
            info!("LN P2P wire service is running on {}", wire_addr);
            wire_service.run_or_panic("LN P2P service").await
        }),
        tokio::spawn(async move {
            info!("Message bus service is listening on {}", bus_addr);
            bus_service.run_loop().await
        })
    )?;

    Ok(())
}

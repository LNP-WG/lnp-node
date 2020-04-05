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


use std::env;
use log::*;
use futures::future;
#[macro_use]
use tokio::try_join;
use tokio::task::JoinHandle;

use lnpd::service::*;
use lnpd::peerd::*;

#[tokio::main]
async fn main() -> Result<(), BootstrapError> {
    println!("\nLNP peerd: Lightning peer daemon; part of Lightning network protocol suite\n");

    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "trace");
    }
    env_logger::init();
    log::set_max_level(LevelFilter::Trace);

    // TODO: Init config from command-line arguments, environment and config file

    let config = Config::default();

    let mut context = zmq::Context::new();

    let lnp2p_service = WireService::init(config.clone().into(), context.clone());
    let subscr_service = BusService::init(config.clone().into(), context.clone());

    let lnp2p_addr = config.lnp2p_addr.clone();
    let subscribe_addr = config.subscribe_addr.clone();

    try_join!(
        tokio::spawn(async move {
            info!("LN P2P service is running on {}", lnp2p_addr);
            lnp2p_service.run_or_panic("LN P2P service").await
        }),
        tokio::spawn(async move {
            info!("Message bus subscription service is listening on {}", subscribe_addr);
            subscr_service.run_loop().await
        })
    );

    Ok(())
}

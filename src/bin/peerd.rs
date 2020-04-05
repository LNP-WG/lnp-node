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
use std::sync::Arc;
use log::*;
use futures::future;
#[macro_use]
use tokio::try_join;
use tokio::task::JoinHandle;
use tokio::net::TcpStream;
#[macro_use]
use clap::clap_app;

use lnpd::conv::*;
use lnpd::service::*;
use lnpd::peerd::*;

#[tokio::main]
async fn main() -> Result<(), BootstrapError> {
    let verify_pubkey = |pubkey_str: String| -> Result<(), String> {
        conv_pubkey(&pubkey_str).map(|_| ())
    };

    let verify_ip = |ip_str: String| -> Result<(), String> {
        conv_ip_port(&ip_str).map(|_| ())
    };

    // TODO: Init config from command-line arguments, environment and config file
    let matches = clap_app!(lbx =>
        (version: "0.1.0")
        (author: "Dr Maxim Orlovsky <orlovsky@pandoracore.com>")
        (about: "LNP peerd: Lightning peer daemon; part of Lightning network protocol suite")
        (@arg verbose: -v ... #{0,2} +global "Sets verbosity level")
        (@arg port: -p --port "Use custom port to bind to (instead of 9735)" )
        (@arg node_id: <node_id> { verify_pubkey } "Public key of the remote node")
        (@arg ip: <ip> +required { verify_ip } "Internet address of the remote node, in form <ip>[:<port>]")
    ).get_matches();

    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "trace");
    }
    env_logger::init();
    log::set_max_level(LevelFilter::Trace);

    let remote_addr = conv_ip_port(
        matches.value_of("node_id").expect("Required parameter absent")
    )?;
    let node_pubkey = conv_pubkey(
        matches.value_of("node_id").expect("Required parameter absent")
    )?;
    let port: u16 = matches.value_of("port")
        .unwrap_or("0")
        .parse()
        .map_err(|_| "Can't parse port number")?;

    let config = Config::default();

    let mut context = zmq::Context::new();

    info!("Connecting to the remote lightning node at {}", remote_addr);
    let mut stream = Arc::new(TcpStream::connect(remote_addr).await?);

    let lnp2p_service = WireService::init(config.clone().into(), context.clone(), stream.clone())?;
    let subscr_service = BusService::init(config.clone().into(), context.clone(), stream.clone())?;

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


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

    let tasks: Vec<JoinHandle<!>> = vec![
        monitor_task
    ].into_iter().flatten().collect();
    future::try_join_all(tasks).await?;

    Ok(())
}

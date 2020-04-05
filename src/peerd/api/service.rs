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


use std::time::Duration;
use tokio::{
    time::delay_for,
    task::JoinHandle
};

use crate::Service;
use super::*;

pub struct BusService {
    config: Config,
    context: zmq::Context,
}

#[async_trait]
impl Service for BusService {
    async fn run_loop(mut self) -> ! {
        loop {
            match self.run().await {
                Ok(_) => debug!("Message bus request processing complete"),
                Err(err) => {
                    error!("Error processing incoming bus message: {}", err)
                },
            }
        }
    }
}

impl BusService {
    pub fn init(config: Config,
                context: zmq::Context) -> Self {
        Self {
            config,
            context,
        }
    }

    async fn run(&mut self) -> Result<(), Error> {
        delay_for(Duration::from_secs(1)).await;
        Ok(())
    }
}

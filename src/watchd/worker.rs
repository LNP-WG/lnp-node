// LNP Node: node running lightning network protocol and generalized lightning
// channels.
// Written in 2020-2022 by
//     Dr. Maxim Orlovsky <orlovsky@lnp-bp.org>
//
// To the extent possible under law, the author(s) have dedicated all
// copyright and related and neighboring rights to this software to
// the public domain worldwide. This software is distributed without
// any warranty.
//
// You should have received a copy of the MIT License along with this software.
// If not, see <https://opensource.org/licenses/MIT>.

use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use bitcoin::hashes::Hash;
use bitcoin::{Transaction, TxMerkleNode, Txid};
use electrum_client::{Client as ElectrumClient, ElectrumApi, HeaderNotification};

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug, Display, Error, From)]
#[display("failed electrum watcher channel")]
#[from(mpsc::SendError<ElectrumCmd>)]
pub struct WatcherChannelFailure;

pub enum ElectrumUpdate {
    Connecting,
    Connected,
    Complete,
    LastBlock(HeaderNotification),
    LastBlockUpdate(HeaderNotification),
    FeeEstimate(f64, f64, f64),
    // TODO: Add TxidMerkleProof to descriptor wallet library
    TxidMerkle(Txid, u32, u32, Vec<TxMerkleNode>),
    TxBatch(Vec<Transaction>, f32),
    ChannelDisconnected,
    Error(electrum_client::Error),
}

pub struct ElectrumWorker {
    worker_thread: JoinHandle<()>,
    pacemaker_thread: JoinHandle<()>,
    tx: mpsc::Sender<ElectrumCmd>,
}

impl ElectrumWorker {
    pub fn with(
        sender: mpsc::Sender<ElectrumUpdate>,
        electrum_url: &str,
        interval: u64,
    ) -> Result<Self, electrum_client::Error> {
        let client = connect_electrum(electrum_url)?;

        let (tx, rx) = mpsc::channel::<ElectrumCmd>();
        let processor = ElectrumProcessor { client, sender, rx };
        let worker_thread = thread::Builder::new()
            .name(s!("electrum_watcher"))
            .spawn(move || processor.run())
            .expect("unable to start blockchain watcher working thread");

        let sender = tx.clone();
        let pacemaker_thread = thread::Builder::new()
            .name(s!("electrum_pacemaker"))
            .spawn(move || loop {
                thread::sleep(Duration::from_secs(interval));
                sender.send(ElectrumCmd::PopHeader).expect("Electrum thread is dead")
            })
            .expect("unable to start blockchain watcher pacemaker thread");

        Ok(ElectrumWorker { tx, worker_thread, pacemaker_thread })
    }

    fn cmd(&self, cmd: ElectrumCmd) -> Result<(), WatcherChannelFailure> {
        self.tx.send(cmd).map_err(WatcherChannelFailure::from)
    }

    #[inline]
    pub fn reconnect(&self, electrum_url: String) -> Result<(), WatcherChannelFailure> {
        self.cmd(ElectrumCmd::Reconnect(electrum_url))
    }

    #[inline]
    pub fn get_merkle(&self, txid: Txid, height: u32) -> Result<(), WatcherChannelFailure> {
        self.cmd(ElectrumCmd::GetTxidMerkle(txid, height))
    }
}

fn connect_electrum(electrum_url: &str) -> Result<ElectrumClient, electrum_client::Error> {
    let config =
        electrum_client::ConfigBuilder::new().timeout(Some(5)).expect("socks are not used").build();
    ElectrumClient::from_config(electrum_url, config)
}

enum ElectrumCmd {
    Reconnect(String),
    PopHeader,
    GetTxidMerkle(Txid, u32),
}

struct ElectrumProcessor {
    client: ElectrumClient,
    sender: mpsc::Sender<ElectrumUpdate>,
    rx: mpsc::Receiver<ElectrumCmd>,
}

impl ElectrumProcessor {
    pub fn run(mut self) {
        loop {
            match self.rx.recv() {
                Ok(cmd) => self.process(cmd),
                Err(_) => {
                    self.sender
                        .send(ElectrumUpdate::ChannelDisconnected)
                        .expect("electrum watcher channel is broken");
                }
            }
        }
    }

    fn process(&mut self, cmd: ElectrumCmd) {
        let resp = match cmd {
            ElectrumCmd::Reconnect(electrum_url) => self.reconnect(&electrum_url).map(|_| None),
            ElectrumCmd::PopHeader => self.pop_header(),
            ElectrumCmd::GetTxidMerkle(txid, height) => self.get_merkle(txid, height).map(Some),
        };
        match resp {
            Ok(Some(msg)) => {
                self.sender.send(msg).expect("electrum watcher channel is broken");
            }
            Ok(None) => { /* nothing to do here */ }
            Err(err) => {
                self.sender
                    .send(ElectrumUpdate::Error(err))
                    .expect("electrum connection is broken");
            }
        }
    }

    fn reconnect(&mut self, electrum_url: &str) -> Result<(), electrum_client::Error> {
        self.client = connect_electrum(electrum_url)?;
        Ok(())
    }

    fn pop_header(&self) -> Result<Option<ElectrumUpdate>, electrum_client::Error> {
        self.client.block_headers_pop().map(|res| res.map(ElectrumUpdate::LastBlockUpdate))
    }

    fn get_merkle(
        &self,
        txid: Txid,
        height: u32,
    ) -> Result<ElectrumUpdate, electrum_client::Error> {
        self.client.transaction_get_merkle(&txid, height as usize).map(|res| {
            let merkle_path = res.merkle.into_iter().map(TxMerkleNode::from_inner).collect();
            ElectrumUpdate::TxidMerkle(txid, res.block_height as u32, res.pos as u32, merkle_path)
        })
    }
}

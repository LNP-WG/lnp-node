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

//! Workflow of launching channeld daemon by lnpd daemon in response to a user request for opening a
//! new channel with a remote peer.
//!
//! MB: This workflow does not cover launching of channeld daemon in response for channel opening
//! request coming from a remote peer, since this is one-stage process and does not require
//! dedicated state machine.

use amplify::{IoError, Slice32, Wrapper};
use bitcoin::Txid;
use lnp::p2p::legacy::{ChannelId, TempChannelId};
use microservices::esb;

use crate::lnpd::funding_wallet;
use crate::lnpd::runtime::Runtime;
use crate::state_machine::{Event, StateMachine};
use crate::{rpc, ServiceId};

/// Errors for channel launching workflow
#[derive(Debug, Display, From, Error)]
#[display(doc_comments)]
pub enum Error {
    /// the received message {0} was not expected at the {1} stage of the channel launch workflow
    UnexpectedMessage(rpc::Request, &'static str),

    /// transaction id changed after signing from {unsigned_txid} to {signed_txid}; may be signd is
    /// hacked
    SignedTxidChanged { unsigned_txid: Txid, signed_txid: Txid },

    /// error sending RPC request during state transition. Details: {0}
    #[from]
    Esb(esb::Error),

    /// error during channel funding
    #[from]
    #[display(inner)]
    Funding(funding_wallet::Error),

    /// unable to launch channel daemon. Details: {0}
    ChannelDaemonLaunch(IoError),
}

/// State machine for launching new channeld by lnpd in response to user channel opening requests.
/// See `doc/workflows/channel_propose.png` for more details.
#[derive(Debug, Display)]
pub enum ChannelLauncher {
    /// Awaiting for channeld to come online and report back to lnpd
    #[display("LAUNCHING")]
    Launching(TempChannelId, rpc::request::CreateChannel),

    /// Awaiting for channeld to complete negotiations on channel structure with the remote peer.
    /// At the end of this state lnpd will construct funding transaction and will provide channeld
    /// with it.
    #[display("NEGOTIATING")]
    Negotiating(TempChannelId),

    /// Awaiting for channeld to sign the commitment transaction with the remote peer. Local
    /// channeld already have the funding transaction received from lnpd at the end of the previous
    /// stage.
    #[display("COMMITTING")]
    Committing(TempChannelId, Txid),

    /// Awaiting signd to sign the funding transaction, after which it can be sent by lnpd to
    /// bitcoin network and the workflow will be complete
    #[display("SIGNING")]
    Signing(ChannelId, Txid),
}

impl StateMachine<rpc::Request, Runtime> for ChannelLauncher {
    type Error = Error;

    fn next(
        self,
        event: Event<rpc::Request>,
        runtime: &mut Runtime,
    ) -> Result<Option<Self>, Self::Error> {
        debug!("ChannelLauncher {} received {} event", self.channel_id(), event.message);
        let channel_id = self.channel_id();
        let state = match self {
            ChannelLauncher::Launching(temp_channel_id, request) => {
                finish_launching(event, temp_channel_id, request)
            }
            ChannelLauncher::Negotiating(temp_channel_id) => {
                finish_negotiating(event, runtime, temp_channel_id)
            }
            ChannelLauncher::Committing(_, txid) => finish_committing(event, runtime, txid),
            ChannelLauncher::Signing(channel_id, txid) => {
                finish_signing(event, runtime, txid)?;
                info!("ChannelLauncher {} has completed its work", channel_id);
                return Ok(None);
            }
        }?;
        info!("ChannelLauncher {} switched to {} state", channel_id, state);
        Ok(Some(state))
    }
}

impl ChannelLauncher {
    /// Computes current channel id for the daemon being launched
    pub fn channel_id(&self) -> Slice32 {
        match self {
            ChannelLauncher::Launching(temp_channel_id, _)
            | ChannelLauncher::Negotiating(temp_channel_id)
            | ChannelLauncher::Committing(temp_channel_id, _) => temp_channel_id.into_inner(),
            ChannelLauncher::Signing(channel_id, _) => channel_id.into_inner(),
        }
    }
}

// State transitions:

impl ChannelLauncher {
    /// Constructs channel launcher state machine
    pub fn with(
        event: Event<rpc::Request>,
        runtime: &Runtime,
        temp_channel_id: TempChannelId,
    ) -> Result<ChannelLauncher, Error> {
        debug!("ChannelLauncher {} is instantiated with {} event", temp_channel_id, event.message);
        let request = match event.message {
            rpc::Request::OpenChannelWith(request) => request,
            msg => {
                panic!("channel_launcher workflow inconsistency: starting workflow with {}", msg)
            }
        };
        runtime
            .launch_channeld(temp_channel_id)
            .map_err(|err| Error::ChannelDaemonLaunch(err.into()))?;
        info!("ChannelLauncher {} entered LAUNCHING state", temp_channel_id);
        Ok(ChannelLauncher::Launching(temp_channel_id, request))
    }
}

fn finish_launching(
    event: Event<rpc::Request>,
    temp_channel_id: TempChannelId,
    request: rpc::request::CreateChannel,
) -> Result<ChannelLauncher, Error> {
    if !matches!(event.message, rpc::Request::Hello) {
        return Err(Error::UnexpectedMessage(event.message, "LAUNCHING"));
    }
    debug_assert_eq!(
        event.source,
        ServiceId::Channel(temp_channel_id.into()),
        "channel_launcher workflow inconsistency: `Hello` RPC CTL message originating not from a \
         channel daemon"
    );
    event.complete_ctl(rpc::Request::OpenChannelWith(request))?;
    Ok(ChannelLauncher::Negotiating(temp_channel_id))
}

fn finish_negotiating(
    event: Event<rpc::Request>,
    runtime: &mut Runtime,
    temp_channel_id: TempChannelId,
) -> Result<ChannelLauncher, Error> {
    let (amount, address, fee) = match event.message {
        rpc::Request::ConstructFunding(rpc::request::FundChannel { amount, address, fee }) => {
            (amount, address, fee)
        }
        _ => {
            return Err(Error::UnexpectedMessage(event.message, "SIGNING"));
        }
    };
    debug_assert_eq!(
        event.source,
        ServiceId::Channel(temp_channel_id.into()),
        "channel_launcher workflow inconsistency: `ConstructFunding` RPC CTL message originating \
         not from a channel daemon"
    );
    let funding_outpoint =
        runtime.funding_wallet.construct_funding_psbt(temp_channel_id, address, amount, fee)?;
    event.complete_ctl(rpc::Request::FundingConstructed(funding_outpoint))?;
    Ok(ChannelLauncher::Committing(temp_channel_id, funding_outpoint.txid))
}

fn finish_committing(
    event: Event<rpc::Request>,
    runtime: &Runtime,
    txid: Txid,
) -> Result<ChannelLauncher, Error> {
    if !matches!(event.message, rpc::Request::PublishFunding) {
        return Err(Error::UnexpectedMessage(event.message, "COMMITTING"));
    }
    let channel_id = if let ServiceId::Channel(channel_id) = event.source {
        channel_id
    } else {
        panic!(
            "channel_launcher workflow inconsistency: `PublishFunding` RPC CTL message \
             originating not from a channel daemon"
        )
    };
    let psbt = runtime
        .funding_wallet
        .get_funding_psbt(txid)
        .expect("funding construction is broken")
        .clone();
    event.complete_ctl_service(ServiceId::Signer, rpc::Request::Sign(psbt))?;
    Ok(ChannelLauncher::Signing(channel_id, txid))
}

fn finish_signing(event: Event<rpc::Request>, runtime: &Runtime, txid: Txid) -> Result<(), Error> {
    let psbt = match event.message {
        rpc::Request::Signed(psbt) => psbt,
        _ => {
            return Err(Error::UnexpectedMessage(event.message, "SIGNING"));
        }
    };
    let psbt_txid = psbt.global.unsigned_tx.txid();
    if psbt_txid != txid {
        return Err(Error::SignedTxidChanged { unsigned_txid: txid, signed_txid: psbt_txid });
    }
    debug_assert_eq!(
        event.source,
        ServiceId::Signer,
        "channel_launcher workflow inconsistency: `Signed` RPC CTL message originating not from a \
         signing daemon"
    );
    runtime.funding_wallet.publish(psbt)?;
    Ok(())
}

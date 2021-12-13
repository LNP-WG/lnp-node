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

use crate::lnpd::runtime::Runtime;
use crate::lnpd::{funding_wallet, Daemon, DaemonError};
use crate::rpc::request::{Failure, OptionDetails, ToProgressOrFalure};
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

    /// failure sending RPC request during state transition. Details: {0}
    #[from]
    Esb(esb::Error),

    /// unable to launch channel daemon. Details: {0}
    #[from(DaemonError<Daemon>)]
    DaemonLaunch(Box<DaemonError<Daemon>>),

    /// failure during channel funding
    #[from]
    #[display(inner)]
    Funding(funding_wallet::Error),
}

/// State machine for launching new channeld by lnpd in response to user channel opening requests.
/// See `doc/workflows/channel_propose.png` for more details.
#[derive(Debug, Display)]
pub enum ChannelLauncher {
    /// Awaiting for channeld to come online and report back to lnpd
    #[display("LAUNCHING")]
    Launching(TempChannelId, rpc::request::CreateChannel, ServiceId),

    /// Awaiting for channeld to complete negotiations on channel structure with the remote peer.
    /// At the end of this state lnpd will construct funding transaction and will provide channeld
    /// with it.
    #[display("NEGOTIATING")]
    Negotiating(TempChannelId, ServiceId),

    /// Awaiting for channeld to sign the commitment transaction with the remote peer. Local
    /// channeld already have the funding transaction received from lnpd at the end of the previous
    /// stage.
    #[display("COMMITTING")]
    Committing(TempChannelId, Txid, ServiceId),

    /// Awaiting signd to sign the funding transaction, after which it can be sent by lnpd to
    /// bitcoin network and the workflow will be complete
    #[display("SIGNING")]
    Signing(ChannelId, Txid, ServiceId),
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
            ChannelLauncher::Launching(temp_channel_id, request, enquirer) => {
                finish_launching(event, runtime, temp_channel_id, request, enquirer)
            }
            ChannelLauncher::Negotiating(temp_channel_id, enquirer) => {
                finish_negotiating(event, runtime, temp_channel_id, enquirer)
            }
            ChannelLauncher::Committing(_, txid, enquirer) => {
                finish_committing(event, runtime, txid, enquirer)
            }
            ChannelLauncher::Signing(channel_id, txid, enquirer) => {
                finish_signing(event, runtime, txid, enquirer)?;
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
            ChannelLauncher::Launching(temp_channel_id, ..)
            | ChannelLauncher::Negotiating(temp_channel_id, ..)
            | ChannelLauncher::Committing(temp_channel_id, ..) => temp_channel_id.into_inner(),
            ChannelLauncher::Signing(channel_id, ..) => channel_id.into_inner(),
        }
    }
}

// State transitions:

impl ChannelLauncher {
    /// Constructs channel launcher state machine
    pub fn with(
        mut event: Event<rpc::Request>,
        runtime: &mut Runtime,
    ) -> Result<ChannelLauncher, Error> {
        let create_channel = match event.message {
            rpc::Request::CreateChannel(ref request) => request.clone(),
            msg => {
                panic!("channel_launcher workflow inconsistency: starting workflow with {}", msg)
            }
        };

        let temp_channel_id = TempChannelId::random();
        debug!("Generated {} as a temporary channel id", temp_channel_id);
        debug!("ChannelLauncher {} is instantiated", temp_channel_id);

        let enquirer = event.source.clone();
        let report = runtime
            .launch_daemon(Daemon::Channeld(temp_channel_id.into()), runtime.config.clone())
            .map(|handle| format!("Launched new instance of {}", handle))
            .map_err(Error::from);
        // Swallowing error since we do not want to break channel creation workflow just because of
        // not able to report back to the client
        let _ = event.send_ctl(report.to_progress_or_failure());
        report?;
        debug!("Awaiting for channeld to connect...");

        info!("ChannelLauncher {} entered LAUNCHING state", temp_channel_id);
        Ok(ChannelLauncher::Launching(temp_channel_id, create_channel, enquirer))
    }
}

fn finish_launching(
    mut event: Event<rpc::Request>,
    runtime: &Runtime,
    temp_channel_id: TempChannelId,
    create_channel: rpc::request::CreateChannel,
    enquirer: ServiceId,
) -> Result<ChannelLauncher, Error> {
    if !matches!(&event.message, rpc::Request::Hello) {
        let err = Error::UnexpectedMessage(event.message.clone(), "LAUNCHING");
        let _ = event.complete_ctl_service(enquirer, Failure::from(&err).into());
        return Err(err);
    }
    debug_assert_eq!(
        event.source,
        ServiceId::Channel(temp_channel_id.into()),
        "channel_launcher workflow inconsistency: `Hello` RPC CTL message originating not from a \
         channel daemon"
    );
    // Swallowing error since we do not want to break channel creation workflow just because of
    // not able to report back to the client
    let _ = event.send_ctl_service(
        enquirer.clone(),
        format!(
            "Channel daemon connecting to remote peer {} is launched",
            create_channel.peerd.clone()
        )
        .into(),
    );
    let mut common = runtime.channel_params.1;
    let mut local = runtime.channel_params.2;
    create_channel.apply_params(&mut common, &mut local);
    let request = rpc::request::OpenChannelWith {
        remote_peer: create_channel.peerd,
        report_to: create_channel.report_to,
        funding_sat: create_channel.funding_sat,
        push_msat: create_channel.push_msat,
        policy: runtime.channel_params.0.clone(),
        common_params: common,
        local_params: local,
        local_keys: runtime.new_channel_keyset(),
    };
    event.send_ctl(rpc::Request::OpenChannelWith(request)).map_err(|err| {
        let _ = event.complete_ctl_service(enquirer.clone(), Failure::from(&err).into());
        err
    })?;
    Ok(ChannelLauncher::Negotiating(temp_channel_id, enquirer))
}

fn finish_negotiating(
    mut event: Event<rpc::Request>,
    runtime: &mut Runtime,
    temp_channel_id: TempChannelId,
    enquirer: ServiceId,
) -> Result<ChannelLauncher, Error> {
    let (amount, address, fee) = match event.message {
        rpc::Request::ConstructFunding(rpc::request::FundChannel { amount, address, fee }) => {
            (amount, address, fee)
        }
        _ => {
            let err = Error::UnexpectedMessage(event.message.clone(), "SIGNING");
            let _ = event.complete_ctl_service(enquirer, Failure::from(&err).into());
            return Err(err);
        }
    };
    debug_assert_eq!(
        event.source,
        ServiceId::Channel(temp_channel_id.into()),
        "channel_launcher workflow inconsistency: `ConstructFunding` RPC CTL message originating \
         not from a channel daemon"
    );
    let _ = event.send_ctl_service(enquirer.clone(), "Remote peer accepted the channel".into());
    let funding_outpoint = runtime
        .funding_wallet
        .construct_funding_psbt(temp_channel_id, address, amount, fee)
        .map_err(Error::from)
        .and_then(|funding_outpoint| {
            event.send_ctl(rpc::Request::FundingConstructed(funding_outpoint)).map(|_| {
                let _ = event.send_ctl_service(
                    enquirer.clone(),
                    rpc::Request::Progress(format!(
                        "Constructed funding transaction with funding outpoint is {}",
                        funding_outpoint
                    )),
                );
            })?;
            Ok(funding_outpoint)
        })
        .map_err(|err| {
            // Swallowing error since we do not want to break channel creation workflow just because
            // of not able to report back to the client
            let _ = event.complete_ctl_service(enquirer.clone(), Failure::from(&err).into());
            err
        })?;
    Ok(ChannelLauncher::Committing(temp_channel_id, funding_outpoint.txid, enquirer))
}

fn finish_committing(
    mut event: Event<rpc::Request>,
    runtime: &Runtime,
    txid: Txid,
    enquirer: ServiceId,
) -> Result<ChannelLauncher, Error> {
    if !matches!(event.message, rpc::Request::PublishFunding) {
        let err = Error::UnexpectedMessage(event.message.clone(), "COMMITTING");
        let _ = event.complete_ctl_service(enquirer, Failure::from(&err).into());
        return Err(err);
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
    let report = event
        .send_ctl_service(ServiceId::Signer, rpc::Request::Sign(psbt))
        .map(|_| format!("Signing funding transaction {}", txid));
    // Swallowing error since we do not want to break channel creation workflow just because of
    // not able to report back to the client
    let _ = event.complete_ctl_service(enquirer.clone(), report.to_progress_or_failure());
    report?;
    Ok(ChannelLauncher::Signing(channel_id, txid, enquirer))
}

fn finish_signing(
    mut event: Event<rpc::Request>,
    runtime: &Runtime,
    txid: Txid,
    enquirer: ServiceId,
) -> Result<(), Error> {
    let psbt = match event.message {
        rpc::Request::Signed(ref psbt) => psbt.clone(),
        _ => {
            let err = Error::UnexpectedMessage(event.message.clone(), "SIGNING");
            let _ = event.complete_ctl_service(enquirer, Failure::from(&err).into());
            return Err(err);
        }
    };
    let psbt_txid = psbt.global.unsigned_tx.txid();
    if psbt_txid != txid {
        let err = Error::SignedTxidChanged { unsigned_txid: txid, signed_txid: psbt_txid };
        let _ = event.complete_ctl_service(enquirer, Failure::from(&err).into());
        return Err(err);
    }
    debug_assert_eq!(
        event.source,
        ServiceId::Signer,
        "channel_launcher workflow inconsistency: `Signed` RPC CTL message originating not from a \
         signing daemon"
    );
    let _ = event.send_ctl_service(
        enquirer.clone(),
        "Funding transaction is signed, publishing to bitcoin network".into(),
    );
    runtime.funding_wallet.publish(psbt)?;
    let _ = event.complete_ctl_service(
        enquirer,
        rpc::Request::Success(OptionDetails::with("Channel created and active")),
    );
    Ok(())
}

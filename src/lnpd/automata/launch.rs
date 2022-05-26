// LNP Node: node running lightning network protocol and generalized lightning
// channels.
// Written in 2020-2022 by
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

use amplify::{Slice32, Wrapper};
use bitcoin::Txid;
use lnp::channel::bolt::LocalKeyset;
use lnp::channel::{FundingError, PsbtLnpFunding};
use lnp::p2p::legacy::{ChannelId, TempChannelId};
use microservices::esb;
use microservices::esb::Handler;

use crate::automata::{Event, StateMachine};
use crate::bus::{BusMsg, CtlMsg, FundChannel, OpenChannelWith, ServiceBus};
use crate::lnpd::runtime::Runtime;
use crate::lnpd::{funding, Daemon, DaemonError};
use crate::rpc::{ClientId, CreateChannel, Failure, OptionDetails, RpcMsg, ServiceId};
use crate::{Endpoints, Responder};

/// Errors for channel launching workflow
#[derive(Debug, Display, From, Error)]
#[display(doc_comments)]
pub enum Error {
    /// the received message {0} was not expected at the {1} stage of the channel launch workflow
    UnexpectedMessage(CtlMsg, &'static str),

    /// transaction id changed after signing from {unsigned_txid} to {signed_txid}; may be signd is
    /// hacked
    SignedTxidChanged { unsigned_txid: Txid, signed_txid: Txid },

    /// incorrect construction of channel funding transaction: {0}
    #[from]
    FundingStructure(FundingError),

    /// failure sending RPC request during state transition. Details: {0}
    #[from]
    Esb(esb::Error<ServiceId>),

    /// unable to launch channel daemon. Details: {0}
    #[from(DaemonError<Daemon>)]
    DaemonLaunch(Box<DaemonError<Daemon>>),

    /// failure during channel funding
    #[from]
    #[display(inner)]
    Funding(funding::Error),
}

impl From<Error> for Failure {
    fn from(err: Error) -> Self { Failure { code: 6000, info: err.to_string() } }
}

/// State machine for launching new channeld by lnpd in response to user channel opening requests.
/// See `doc/workflows/channel_propose.png` for more details.
///
/// State machine workflow:
/// ```ignore
///           START
///             |
///        +---------+
///        V         V
///    LAUNCHING  DERIVING
///        |         |
///        +---------+
///             V
///        NEGOTIATING
///             |
///             V
///        COMMITTING
///             |
///             V
///          SIGNING
///             |
///             V
///           DONE
/// ```
#[derive(Clone, PartialEq, Eq, Debug, Display, StrictEncode, StrictDecode)]
pub enum ChannelLauncher {
    /// Awaiting for channeld to come online and report back to lnpd + for signd to derive keyset
    /// in parallel.
    #[display("INIT")]
    Init(TempChannelId, CreateChannel, ClientId),

    /// Keyset for channel is derived. Still awaiting for channeld to come online and report back
    /// to lnpd.
    #[display("LAUNCHING")]
    Launching(TempChannelId, CreateChannel, ClientId, LocalKeyset),

    /// Channel daemon is launched, awaiting for keyset to be derived.
    #[display("DERIVING")]
    Deriving(TempChannelId, CreateChannel, ClientId),

    /// Awaiting for channeld to complete negotiations on channel structure with the remote peer.
    /// At the end of this state lnpd will construct funding transaction and will provide channeld
    /// with it.
    #[display("NEGOTIATING")]
    Negotiating(TempChannelId, ClientId),

    /// Awaiting for channeld to sign the commitment transaction with the remote peer. Local
    /// channeld already have the funding transaction received from lnpd at the end of the previous
    /// stage.
    #[display("COMMITTING")]
    Committing(ChannelId, Txid, ClientId),

    /// Awaiting signd to sign the funding transaction, after which it can be sent by lnpd to
    /// bitcoin network and the workflow will be complete
    #[display("SIGNING")]
    Signing(ChannelId, Txid, ClientId),
}

impl StateMachine<CtlMsg, Runtime> for ChannelLauncher {
    type Error = Error;

    fn next(
        self,
        event: Event<CtlMsg>,
        runtime: &mut Runtime,
    ) -> Result<Option<Self>, Self::Error> {
        debug!("ChannelLauncher {:#} received {} event", self.channel_id(), event.message);
        let channel_id = self.channel_id();
        if let CtlMsg::Error { error, .. } = &event.message {
            let failure = Failure { code: 10000, info: error.clone() };
            runtime.send_rpc(event.endpoints, self.enquirer(), RpcMsg::Failure(failure))?;
            return Ok(None);
        }
        let state = match self {
            ChannelLauncher::Init(temp_channel_id, request, enquirer) => match event.message {
                CtlMsg::Hello => complete_launch(event, temp_channel_id, request, enquirer),
                CtlMsg::Keyset(_, ref keyset) => {
                    let keyset = keyset.clone();
                    complete_derivation(event, temp_channel_id, keyset, request, enquirer)
                }
                _ => {
                    let err = Error::UnexpectedMessage(event.message.clone(), "INIT");
                    report_failure(enquirer, event.endpoints, err)?;
                    unreachable!()
                }
            },
            ChannelLauncher::Deriving(temp_channel_id, request, enquirer) => {
                start_negotiation1(event, runtime, temp_channel_id, request, enquirer)
            }
            ChannelLauncher::Launching(temp_channel_id, request, enquirer, keyset) => {
                start_negotiation2(event, runtime, temp_channel_id, keyset, request, enquirer)
            }
            ChannelLauncher::Negotiating(temp_channel_id, enquirer) => {
                complete_negotiation(event, runtime, temp_channel_id, enquirer)
            }
            ChannelLauncher::Committing(_, ref txid, ref enquirer) => {
                match event.message {
                    // Since we changed channeld id we send hello request once again, but this does
                    // not influence state machine
                    CtlMsg::Hello => Ok(self),
                    _ => complete_commitment(event, runtime, *txid, *enquirer),
                }
            }
            ChannelLauncher::Signing(channel_id, txid, enquirer) => {
                complete_signatures(event, runtime, txid, enquirer)?;
                info!("ChannelLauncher {:#} has completed its work", channel_id);
                return Ok(None);
            }
        }?;
        info!("ChannelLauncher {:#} switched to {} state", channel_id, state);
        Ok(Some(state))
    }
}

impl ChannelLauncher {
    /// Computes current channel id for the daemon being launched
    pub fn channel_id(&self) -> Slice32 {
        match self {
            ChannelLauncher::Init(temp_channel_id, ..)
            | ChannelLauncher::Launching(temp_channel_id, ..)
            | ChannelLauncher::Deriving(temp_channel_id, ..)
            | ChannelLauncher::Negotiating(temp_channel_id, ..) => temp_channel_id.into_inner(),
            ChannelLauncher::Committing(channel_id, ..)
            | ChannelLauncher::Signing(channel_id, ..) => channel_id.into_inner(),
        }
    }

    pub fn funding_txid(&self) -> Option<Txid> {
        match self {
            ChannelLauncher::Init(_, _, _)
            | ChannelLauncher::Launching(_, _, _, _)
            | ChannelLauncher::Deriving(_, _, _)
            | ChannelLauncher::Negotiating(_, _) => None,
            ChannelLauncher::Committing(_, txid, _) | ChannelLauncher::Signing(_, txid, _) => {
                Some(*txid)
            }
        }
    }

    pub fn enquirer(&self) -> ClientId {
        match self {
            ChannelLauncher::Init(_, _, enquirer)
            | ChannelLauncher::Launching(_, _, enquirer, _)
            | ChannelLauncher::Deriving(_, _, enquirer)
            | ChannelLauncher::Negotiating(_, enquirer)
            | ChannelLauncher::Committing(_, _, enquirer)
            | ChannelLauncher::Signing(_, _, enquirer) => *enquirer,
        }
    }
}

// State transitions:

impl ChannelLauncher {
    /// Constructs channel launcher state machine
    pub fn with(
        endpoints: &mut Endpoints,
        enquirer: ClientId,
        create_channel: CreateChannel,
        runtime: &mut Runtime,
    ) -> Result<ChannelLauncher, Error> {
        let temp_channel_id = TempChannelId::random();
        debug!("Generated {} as a temporary channel id", temp_channel_id);
        debug!("ChannelLauncher {:#} is instantiated", temp_channel_id);

        let report = runtime
            .launch_daemon(Daemon::Channeld(temp_channel_id.into()), runtime.config.clone())
            .map(|handle| format!("Launched new instance of {}", handle))
            .map_err(Error::from);
        report_progress_or_failure(enquirer, endpoints, report)?;

        debug!("Asking signd to derive keyset for the channel {}", temp_channel_id);
        let report = endpoints
            .send_to(
                ServiceBus::Ctl,
                runtime.identity(),
                ServiceId::Signer,
                BusMsg::Ctl(CtlMsg::DeriveKeyset(temp_channel_id.into_inner())),
            )
            .map(|_| s!("Deriving basepoint keys for the channel"))
            .map_err(Error::from);
        report_progress_or_failure(enquirer, endpoints, report)?;

        let launcher = ChannelLauncher::Init(temp_channel_id, create_channel, enquirer);
        debug!("Awaiting for channeld to connect...");

        info!("ChannelLauncher {:#} entered LAUNCHING state", temp_channel_id);
        Ok(launcher)
    }
}

fn complete_launch(
    event: Event<CtlMsg>,
    temp_channel_id: TempChannelId,
    create_channel: CreateChannel,
    enquirer: ClientId,
) -> Result<ChannelLauncher, Error> {
    debug_assert_eq!(
        event.source,
        ServiceId::Channel(temp_channel_id.into()),
        "channel_launcher workflow inconsistency: `Hello` RPC CTL message originating not from a \
         channel daemon"
    );
    report_progress(
        enquirer,
        event.endpoints,
        format!(
            "Channel daemon connecting to remote peer {} is launched",
            create_channel.remote_peer
        ),
    );
    Ok(ChannelLauncher::Deriving(temp_channel_id, create_channel, enquirer))
}

fn complete_derivation(
    event: Event<CtlMsg>,
    temp_channel_id: TempChannelId,
    keyset: LocalKeyset,
    create_channel: CreateChannel,
    enquirer: ClientId,
) -> Result<ChannelLauncher, Error> {
    debug_assert_eq!(
        event.source,
        ServiceId::Signer,
        "channel_launcher workflow inconsistency: `Keyset` RPC CTL message originating not from a \
         sign daemon"
    );
    report_progress(enquirer, event.endpoints, "Key derivation complete");
    Ok(ChannelLauncher::Launching(temp_channel_id, create_channel, enquirer, keyset))
}

fn start_negotiation1(
    event: Event<CtlMsg>,
    runtime: &Runtime,
    temp_channel_id: TempChannelId,
    create_channel: CreateChannel,
    enquirer: ClientId,
) -> Result<ChannelLauncher, Error> {
    debug_assert_eq!(
        event.source,
        ServiceId::Signer,
        "channel_launcher workflow inconsistency: `Keyset` RPC CTL message originating not from a \
         sign daemon"
    );
    let keyset = match &event.message {
        CtlMsg::Keyset(_, keyset) => keyset.clone(),
        _ => {
            let err = Error::UnexpectedMessage(event.message.clone(), "LAUNCHING");
            report_failure(enquirer, event.endpoints, err)?;
            unreachable!()
        }
    };
    report_progress(enquirer, event.endpoints, "Key derivation complete");
    start_negotiation(event, runtime, temp_channel_id, keyset, create_channel, enquirer)
}

fn start_negotiation2(
    event: Event<CtlMsg>,
    runtime: &Runtime,
    temp_channel_id: TempChannelId,
    keyset: LocalKeyset,
    create_channel: CreateChannel,
    enquirer: ClientId,
) -> Result<ChannelLauncher, Error> {
    if !matches!(event.message, CtlMsg::Hello) {
        let err = Error::UnexpectedMessage(event.message.clone(), "DERIVING");
        report_failure(enquirer, event.endpoints, err)?;
        unreachable!()
    }
    report_progress(
        enquirer,
        event.endpoints,
        format!(
            "Channel daemon connecting to remote peer {} is launched",
            create_channel.remote_peer
        ),
    );
    start_negotiation(event, runtime, temp_channel_id, keyset, create_channel, enquirer)
}

fn start_negotiation(
    mut event: Event<CtlMsg>,
    runtime: &Runtime,
    temp_channel_id: TempChannelId,
    keyset: LocalKeyset,
    create_channel: CreateChannel,
    enquirer: ClientId,
) -> Result<ChannelLauncher, Error> {
    let mut common = runtime.channel_params.1;
    let mut local = runtime.channel_params.2;
    create_channel.apply_params(&mut common, &mut local);
    let request = OpenChannelWith {
        remote_peer: create_channel.remote_peer,
        report_to: create_channel.report_to,
        funding_sat: create_channel.funding_sat,
        push_msat: create_channel.push_msat,
        policy: runtime.channel_params.0.clone(),
        common_params: common,
        local_params: local,
        local_keys: keyset,
    };
    event
        .send_ctl(CtlMsg::OpenChannelWith(request))
        .or_else(|err| report_failure(enquirer, event.endpoints, Error::from(err)))?;
    Ok(ChannelLauncher::Negotiating(temp_channel_id, enquirer))
}

fn complete_negotiation(
    mut event: Event<CtlMsg>,
    runtime: &mut Runtime,
    temp_channel_id: TempChannelId,
    enquirer: ClientId,
) -> Result<ChannelLauncher, Error> {
    let (amount, script_pubkey, feerate_per_kw) = match event.message {
        CtlMsg::ConstructFunding(FundChannel { amount, ref script_pubkey, feerate_per_kw }) => {
            (amount, script_pubkey, feerate_per_kw)
        }
        _ => {
            let err = Error::UnexpectedMessage(event.message.clone(), "SIGNING");
            report_failure(enquirer, event.endpoints, err)?;
            unreachable!()
        }
    };
    debug_assert_eq!(
        event.source,
        ServiceId::Channel(temp_channel_id.into()),
        "channel_launcher workflow inconsistency: `ConstructFunding` RPC CTL message originating \
         not from a channel daemon"
    );
    report_progress(enquirer, event.endpoints, "Remote peer accepted the channel");
    let funding_outpoint = runtime
        .funding_wallet
        .construct_funding_psbt(temp_channel_id, script_pubkey.clone(), amount, feerate_per_kw)
        .map_err(Error::from)
        .and_then(|psbt| {
            let funding_outpoint = psbt.channel_funding_outpoint()?;
            event.send_ctl(CtlMsg::FundingConstructed(psbt)).map(|_| {
                report_progress(
                    enquirer,
                    event.endpoints,
                    format!(
                        "Constructed funding transaction with funding outpoint {}",
                        funding_outpoint
                    ),
                );
            })?;
            Ok(funding_outpoint)
        })
        .map_err(|err| report_failure(enquirer, event.endpoints, err).unwrap_err())?;

    let channel_id = ChannelId::with(funding_outpoint.txid, funding_outpoint.vout as u16);
    runtime.update_chanel_id(temp_channel_id, channel_id);
    report_progress(
        enquirer,
        event.endpoints,
        format!(
            "Channel changed id from temporary {} to permanent {}",
            temp_channel_id, channel_id
        ),
    );

    Ok(ChannelLauncher::Committing(channel_id, funding_outpoint.txid, enquirer))
}

fn complete_commitment(
    mut event: Event<CtlMsg>,
    runtime: &Runtime,
    txid: Txid,
    enquirer: ClientId,
) -> Result<ChannelLauncher, Error> {
    if !matches!(event.message, CtlMsg::PublishFunding) {
        let err = Error::UnexpectedMessage(event.message.clone(), "COMMITTING");
        report_failure(enquirer, event.endpoints, err)?;
        unreachable!()
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
        .send_ctl_service(ServiceId::Signer, CtlMsg::Sign(psbt))
        .map(|_| format!("Signing funding transaction {}", txid))
        .map_err(Error::from);
    report_progress_or_failure(enquirer, event.endpoints, report)?;
    Ok(ChannelLauncher::Signing(channel_id, txid, enquirer))
}

fn complete_signatures(
    event: Event<CtlMsg>,
    runtime: &Runtime,
    txid: Txid,
    enquirer: ClientId,
) -> Result<(), Error> {
    let funding_psbt = match event.message {
        CtlMsg::Signed(ref psbt) => psbt.clone(),
        _ => {
            let err = Error::UnexpectedMessage(event.message.clone(), "SIGNING");
            report_failure(enquirer, event.endpoints, err)?;
            unreachable!();
        }
    };

    let psbt_txid = funding_psbt.to_txid();
    if psbt_txid != txid {
        let err = Error::SignedTxidChanged { unsigned_txid: txid, signed_txid: psbt_txid };
        report_failure(enquirer, event.endpoints, err)?;
        unreachable!()
    }
    debug_assert_eq!(
        event.source,
        ServiceId::Signer,
        "channel_launcher workflow inconsistency: `Signed` RPC CTL message originating not from a \
         signing daemon"
    );
    report_progress(
        enquirer,
        event.endpoints,
        "Funding transaction is signed, publishing to bitcoin network",
    );
    runtime.funding_wallet.publish(funding_psbt)?;
    report_success(enquirer, event.endpoints, "Channel created and active");
    Ok(())
}

fn report_failure<E>(client_id: ClientId, endpoints: &mut Endpoints, err: E) -> Result<(), Error>
where
    E: Into<Failure> + Into<Error> + std::error::Error,
{
    let enquirer = ServiceId::Client(client_id);
    let report = RpcMsg::Failure(Failure::from(&err));
    // Swallowing error since we do not want to break channel creation workflow just because of
    // not able to report back to the client
    let _ = endpoints
        .send_to(ServiceBus::Rpc, ServiceId::LnpBroker, enquirer, BusMsg::Rpc(report))
        .map_err(|err| error!("Can't report back to client #{}: {}", client_id, err));
    Err(err.into())
}

fn report_progress<T>(client_id: ClientId, endpoints: &mut Endpoints, msg: T)
where
    T: ToString,
{
    let enquirer = ServiceId::Client(client_id);
    let report = RpcMsg::Progress(msg.to_string());
    // Swallowing error since we do not want to break channel creation workflow just because of
    // not able to report back to the client
    let _ = endpoints
        .send_to(ServiceBus::Rpc, ServiceId::LnpBroker, enquirer, BusMsg::Rpc(report))
        .map_err(|err| error!("Can't report back to client #{}: {}", client_id, err));
}

fn report_success<T>(client_id: ClientId, endpoints: &mut Endpoints, msg: T)
where
    T: Into<OptionDetails>,
{
    let enquirer = ServiceId::Client(client_id);
    let report = RpcMsg::Success(msg.into());
    // Swallowing error since we do not want to break channel creation workflow just because of
    // not able to report back to the client
    let _ = endpoints
        .send_to(ServiceBus::Rpc, ServiceId::LnpBroker, enquirer, BusMsg::Rpc(report))
        .map_err(|err| error!("Can't report back to client #{}: {}", client_id, err));
}

fn report_progress_or_failure<T, E>(
    client_id: ClientId,
    endpoints: &mut Endpoints,
    result: Result<T, E>,
) -> Result<(), Error>
where
    T: ToString,
    E: Into<Failure> + Into<Error> + std::error::Error,
{
    let enquirer = ServiceId::Client(client_id);
    let report = match &result {
        Ok(val) => RpcMsg::Progress(val.to_string()),
        Err(err) => RpcMsg::Failure(err.into()),
    };
    // Swallowing error since we do not want to break channel creation workflow just because of
    // not able to report back to the client
    let _ = endpoints
        .send_to(ServiceBus::Rpc, ServiceId::LnpBroker, enquirer, BusMsg::Rpc(report))
        .map_err(|err| error!("Can't report back to client #{}: {}", client_id, err));
    result.map(|_| ()).map_err(E::into)
}

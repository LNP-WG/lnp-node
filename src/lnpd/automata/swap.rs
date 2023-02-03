//! Workflow for launching swapd daemon by lnpd daemon in response to a user request for starting a
//! new swap with a remote peer.

use std::fs;

use bitcoin::secp256k1::{self, rand, Secp256k1};
use bitcoin::util::bip32::{self, ChildNumber, ExtendedPrivKey};
use bitcoin::{KeyPair, Txid};
use internet2::addr::NodeId;
use lnp::p2p::bifrost::{self, SwapId, SwapOutRequestMsg};
use lnp::p2p::bolt;
use lnp_rpc::{
    ChannelInfo, Failure, FailureCode, NodeOrChannelId, RpcMsg, ServiceId, SwapIn, SwapOut,
};
use lnpbp::chain::{AssetId, Chain, ConversionImpossibleError};
use microservices::esb::ClientId;
use microservices::{esb, LauncherError};
use strict_encoding::StrictDecode;
use wallet::hd::UnhardenedIndex;

use crate::automata::StateMachine;
use crate::bus::{BusMsg, CtlMsg, ServiceBus};
use crate::lnpd::automata::report::{report_progress, report_progress_or_failure};
use crate::lnpd::runtime::Runtime;
use crate::lnpd::Daemon;
use crate::{Config, Endpoints, Responder, LNP_NODE_SWAP_KEY};

#[derive(Clone, Debug, StrictEncode, StrictDecode)]
struct SwapKeyData {
    pub xpriv: ExtendedPrivKey,
    pub last_index: UnhardenedIndex,
}

impl SwapKeyData {
    fn get_next_keypair(
        &self,
        chain_index: u32,
        secp: &Secp256k1<secp256k1::All>,
    ) -> Result<KeyPair, Error> {
        let mut path = [chain_index, 1, 0]
            .iter()
            .map(|idx| ChildNumber::from_hardened_idx(*idx).expect("hardcoded index"))
            .collect::<Vec<_>>();

        path.push(self.last_index.into());

        let key_pair = self
            .xpriv
            .derive_priv(secp, &path)
            .map(|xpriv| xpriv.to_keypair(secp))
            .map_err(Error::from)?;

        Ok(key_pair)
    }
}

impl Config {
    fn read_swap_key(&self) -> Result<SwapKeyData, Error> {
        let mut swap_key_path = self.data_dir.clone();
        swap_key_path.push(LNP_NODE_SWAP_KEY);

        let wallet_file =
            fs::OpenOptions::new().read(true).write(true).create(false).open(swap_key_path)?;
        let key_data = SwapKeyData::strict_decode(&wallet_file)?;
        Ok(key_data)
    }

    fn create_swap_key(&self) -> Result<SwapKeyData, Error> {
        let mut swap_key_path = self.data_dir.clone();
        swap_key_path.push(LNP_NODE_SWAP_KEY);
        info!("Creating swap intermediate key file at '{}'", swap_key_path.display());
        let wallet_file = fs::File::create(swap_key_path)?;

        let seed: Vec<u8> = (0..64).map(|_| rand::random::<u8>()).collect();

        let network: bitcoin::Network = self.chain.clone().try_into()?;
        let data = SwapKeyData {
            xpriv: ExtendedPrivKey::new_master(network, &seed)?,
            last_index: <UnhardenedIndex as wallet::hd::SegmentIndexes>::zero(),
        };
        Ok(data)
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Display, NetworkEncode, NetworkDecode)]
#[display(Debug)]
pub enum SwapRequest {
    Out(SwapOut),
    In(SwapIn),
}

/// Errors for channel launching workflow
#[derive(Debug, Display, From, Error)]
#[display(doc_comments)]
pub enum ValidationError {
    Foo,
}

impl SwapRequest {
    /// Validate request against current channel state.
    /// Returns most plausible channel id to swap in case of success.
    fn validate(
        &self,
        channel_infos: &Vec<ChannelInfo>,
    ) -> Result<bolt::ChannelId, ValidationError> {
        todo!()
    }

    fn node_or_channel_id(&self) -> &NodeOrChannelId {
        match self {
            Self::Out(SwapOut { node_or_chan_id, .. })
            | Self::In(SwapIn { node_or_chan_id, .. }) => node_or_chan_id,
        }
    }

    fn asset(&self) -> &Option<AssetId> {
        match self {
            Self::Out(SwapOut { asset, .. }) | Self::In(SwapIn { asset, .. }) => asset,
        }
    }

    fn amount(&self) -> u64 {
        match self {
            Self::Out(SwapOut { amount, .. }) | Self::In(SwapIn { amount, .. }) => *amount,
        }
    }
}

/// Errors for channel launching workflow
#[derive(Debug, Display, From, Error)]
#[display(doc_comments)]
pub enum Error {
    /// the received message {0} was not expected at the {1} stage of the channel launch workflow
    UnexpectedMessage(CtlMsg, &'static str),

    /// unable to launch swap daemon. Details: {0}
    #[from(LauncherError<Daemon>)]
    DaemonLaunch(Box<LauncherError<Daemon>>),

    /// failure sending RPC request during state transition. Details: {0}
    #[from]
    Esb(esb::Error<ServiceId>),

    /// Failed to open file
    #[from]
    FileIo(std::io::Error),

    /// Failed to decode file
    #[from]
    Encoding(strict_encoding::Error),

    /// RPC request was invalid
    #[from]
    InvalidRequest(ValidationError),

    /// swap key creation error.
    #[from]
    BIP32(bip32::Error),

    /// could not create network from chain
    #[from]
    InvalidChain(ConversionImpossibleError),

    /// could not retrieve NodeId from peerd/channeld
    NodeIdNotFound,
}

impl From<&Error> for Failure {
    fn from(err: &Error) -> Self { Failure { code: FailureCode::Swap, info: err.to_string() } }
}
impl From<Error> for Failure {
    fn from(err: Error) -> Self { Failure { code: FailureCode::Swap, info: err.to_string() } }
}

/// State machine for launching new swap by swapd in response to user swap request.
///
/// State machine workflow:
/// ```ignore
///           INIT
///             |
///             --------+
///             |       V
///             |   AWAITING_PEER_INFO
///             |       |
///             +-------+
///             |
///     AWAITING_CHANNEL_INFOS
///             |
///             V
///        NEGOTIATING
///             |
///             V
///          SIGNING
///             |
///             V
///           DONE
/// ```
#[derive(Clone, Debug, Display, StrictEncode, StrictDecode)]
pub enum SwapLauncherState {
    /// Awaiting for swapd to come online and report back to lnpd + for signd to derive keyset
    /// in parallel.
    #[display("INIT")]
    Init(SwapId, SwapRequest, ClientId),

    /// Waiting peerd to report current status of the pee
    /// (most importantly, channel ids we have against them).
    #[display("AWAITING_PEER_INFO")]
    AwaitingPeerInfo(SwapId, SwapRequest, ClientId),

    /// Waiting channeld to report current channel information against the peer.
    #[display("AWAITING_CHANNEL_INFOS")]
    AwaitingChannelInfos {
        swap_id: SwapId,
        num_expecting: u16,
        infos_sofar: Vec<ChannelInfo>,
        request: SwapRequest,
        node_id: Option<NodeId>,
        enquirer: ClientId,
    },

    /// Awaiting for swapd to complete negotiations on swap with the remote peer.
    /// At the end of this state lnpd will construct swap either
    /// 1. transaction (swap in)
    /// 2. off-chain payment (swap out)
    #[display("NEGOTIATING")]
    Negotiating(SwapId, ClientId),

    /// Awaiting for swapd to sign the construct transaction, after which it can be sent by lnpd to
    /// bitcoin network and the workflow will be complete.
    #[display("SIGNING")]
    Signing(SwapId, Txid, ClientId),
}

pub struct SwapLauncher {
    state: SwapLauncherState,
    chain: Chain,
    key_data: SwapKeyData,
    secp: Secp256k1<secp256k1::All>,
}

impl StateMachine<CtlMsg, Runtime> for SwapLauncher {
    type Error = Error;

    fn next(
        self,
        event: crate::automata::Event<CtlMsg>,
        runtime: &mut Runtime,
    ) -> Result<Option<Self>, Self::Error>
    where
        Self: Sized,
    {
        debug!("SwapLauncher {:#} received {} event", self.swap_id(), event.message);
        if let CtlMsg::Error { destination, request, error } = event.message {
            let failure = Failure { code: FailureCode::Swap, info: error.clone() };
            runtime.send_rpc(event.endpoints, self.enquirer(), RpcMsg::Failure(failure))?;
            return Ok(None);
        }

        let next_state = match self.state {
            SwapLauncherState::Init(swap_id, request, enquirer) => {
                match event.message {
                    CtlMsg::Hello => {
                        debug_assert_eq!(
                            event.source,
                            ServiceId::Swapd(swap_id.clone()),
                            "swapd_launcher workflow inconsistency: `Hello` RPC CTL message \
                             originating not from a swap daemon"
                        );
                        report_progress(
                            enquirer,
                            event.endpoints,
                            format!("Swap daemon for {} launched.", request.node_or_channel_id()),
                        );

                        // Ask fellow microservices for information necessary to construct p2p
                        // messages.
                        match request.node_or_channel_id() {
                            NodeOrChannelId::ChannelId(c) => {
                                let mut e = event;
                                e.send_ctl_service(ServiceId::Channel(*c), CtlMsg::GetInfo)?;
                                Some(SwapLauncherState::AwaitingChannelInfos {
                                    swap_id,
                                    num_expecting: 1,
                                    infos_sofar: Vec::with_capacity(1),
                                    request,
                                    node_id: None,
                                    enquirer,
                                })
                            }
                            NodeOrChannelId::NodeId(node_id) => {
                                let mut e = event;
                                e.send_ctl_service(
                                    ServiceId::PeerBifrost(*node_id),
                                    CtlMsg::GetInfo,
                                )?;
                                Some(SwapLauncherState::AwaitingPeerInfo(
                                    swap_id, request, enquirer,
                                ))
                            }
                        }
                    }
                    _ => todo!(),
                }
            }

            SwapLauncherState::AwaitingPeerInfo(swap_id, request, enquirer) => {
                match event.message {
                    CtlMsg::PeerInfo(_) => {
                        // dirty hack to circumvent the rust lifetime checker
                        let mut dest: Vec<bolt::ChannelId> = none!();
                        let mut len = 0;
                        let mut remote_id = None;
                        if let CtlMsg::PeerInfo(ref info) = event.message {
                            remote_id = Some(info.remote_id[0]);
                            len = (&info.channels).len();
                            for c in &info.channels {
                                let id = bolt::ChannelId::from(c.clone());
                                dest.push(id);
                            }
                        }
                        let mut e = event;
                        for id in dest {
                            &e.send_ctl_service(ServiceId::Channel(id), CtlMsg::GetInfo)?;
                        }
                        Some(SwapLauncherState::AwaitingChannelInfos {
                            swap_id,
                            num_expecting: len as u16,
                            infos_sofar: Vec::with_capacity(len),
                            request,
                            node_id: remote_id,
                            enquirer,
                        })
                    }
                    // do nothing
                    _ => None,
                }
            }

            SwapLauncherState::AwaitingChannelInfos {
                swap_id,
                num_expecting,
                infos_sofar,
                request,
                node_id,
                enquirer,
            } => {
                match event.message {
                    CtlMsg::ChannelInfo(info) => {
                        let mut infos_sofar = infos_sofar.clone();
                        infos_sofar.push(info);

                        if num_expecting == infos_sofar.len() as u16 {
                            let chan_id = request.validate(&infos_sofar)?;

                            let node_id = {
                                let mut maybe_id = None;
                                for i in infos_sofar {
                                    if i.remote_id.is_some() {
                                        maybe_id = i.remote_id;
                                        break;
                                    }
                                }
                                node_id.and(maybe_id).ok_or(Error::NodeIdNotFound)
                            }?;

                            let key_pair = &self.key_data.get_next_keypair(
                                self.chain.chain_params().is_testnet as u32,
                                &self.secp,
                            )?;
                            let req = SwapOutRequestMsg {
                                protocol_version: lnp::p2p::bifrost::PROTOCOL_VERSION as u64,
                                swap_id: swap_id.clone(),
                                asset: request.asset().clone(),
                                network: runtime.config.chain.to_string(),
                                scid: chan_id,
                                amount: request.amount(),
                                pubkey: key_pair.public_key(),
                            };
                            let message = BusMsg::Bifrost(bifrost::Messages::SwapOutRequest(req));
                            event.endpoints.send_to(
                                ServiceBus::Ctl,
                                event.source,
                                ServiceId::PeerBifrost(node_id),
                                message,
                            );
                            Some(SwapLauncherState::Negotiating(swap_id, enquirer))
                        } else {
                            None
                        }
                    }
                    // do nothing
                    _ => None,
                }
            }

            SwapLauncherState::Negotiating(swap_id, enquirer) => {
                todo!()
            }
            SwapLauncherState::Signing(_, _, _) => todo!(),
        };

        Ok(next_state.map(|state| SwapLauncher { state, ..self }))
    }
}

impl SwapLauncher {
    pub fn with(
        endpoints: &mut Endpoints,
        enquirer: ClientId,
        request: SwapRequest,
        runtime: &mut Runtime,
    ) -> Result<SwapLauncher, Error> {
        let swap_id = SwapId::random();
        debug!("SwapLauncher with id {} is instantiated", swap_id);

        let report = runtime
            .launch_daemon(Daemon::Swapd(swap_id.clone()), runtime.config.clone())
            .map(|handle| format!("Launched new instance of {}", handle))
            .map_err(Error::from);
        report_progress_or_failure(enquirer, endpoints, report)?;

        // prepare keys

        let state = SwapLauncherState::Init(swap_id, request, enquirer);
        let launcher = SwapLauncher {
            state,
            chain: runtime.config.chain.clone(),
            key_data: todo!(),
            secp: todo!(),
        };

        // prepare messages
        info!("SwapLauncher {:#} entered LAUNCHING state", swap_id);
        Ok(launcher)
    }

    fn enquirer(&self) -> ClientId { self.state.enquirer() }
    pub fn swap_id(&self) -> SwapId { self.state.swap_id() }
}

impl SwapLauncherState {
    pub fn swap_id(&self) -> SwapId {
        match self {
            Self::Init(swap_id, _, _)
            | Self::AwaitingPeerInfo(swap_id, _, _)
            | Self::AwaitingChannelInfos { swap_id, .. }
            | Self::Negotiating(swap_id, _)
            | Self::Signing(swap_id, _, _) => swap_id.clone(),
        }
    }

    pub fn enquirer(&self) -> ClientId {
        match self {
            Self::Init(_, _, enquirer)
            | Self::AwaitingPeerInfo(_, _, enquirer)
            | Self::AwaitingChannelInfos { enquirer, .. }
            | Self::Negotiating(_, enquirer)
            | Self::Signing(_, _, enquirer) => *enquirer,
        }
    }
}

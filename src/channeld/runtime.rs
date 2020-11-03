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

use amplify::DumbDefault;
use std::collections::BTreeMap;
use std::time::{Duration, SystemTime};

use lnpbp::bitcoin::secp256k1::{self, Signature};
use lnpbp::bitcoin::OutPoint;
use lnpbp::bp::{Chain, PubkeyScript};
use lnpbp::lnp::application::channel::ScriptGenerators;
use lnpbp::lnp::{
    message, AssetsBalance, ChannelId, ChannelKeys, ChannelNegotiationError,
    ChannelParams, ChannelState, Messages, NodeAddr, TempChannelId, TypedEnum,
};
use lnpbp_services::esb::{self, Handler};

use crate::rpc::request::ChannelInfo;
use crate::rpc::{request, Request, ServiceBus};
use crate::{Config, CtlServer, Error, LogStyle, Senders, Service, ServiceId};

pub fn run(
    config: Config,
    node_id: secp256k1::PublicKey,
    channel_id: ChannelId,
    chain: Chain,
) -> Result<(), Error> {
    let runtime = Runtime {
        identity: ServiceId::Channel(channel_id),
        peer_service: ServiceId::Loopback,
        node_id,
        chain,
        channel_id: None,
        temporary_channel_id: channel_id.into(),
        state: default!(),
        local_capacity: 0,
        remote_capacity: 0,
        local_balances: zero!(),
        remote_balances: zero!(),
        funding_outpoint: default!(),
        remote_peer: None,
        started: SystemTime::now(),
        total_updates: 0,
        pending_updates: 0,
        params: default!(),
        local_keys: dumb!(),
        remote_keys: dumb!(),
        enquirer: None,
    };

    Service::run(config, runtime, false)
}

pub struct Runtime {
    identity: ServiceId,
    peer_service: ServiceId,
    node_id: secp256k1::PublicKey,
    chain: Chain,

    channel_id: Option<ChannelId>,
    temporary_channel_id: TempChannelId,
    state: ChannelState,
    local_capacity: u64,
    remote_capacity: u64,
    local_balances: AssetsBalance,
    remote_balances: AssetsBalance,
    funding_outpoint: OutPoint,
    remote_peer: Option<NodeAddr>,
    started: SystemTime,
    total_updates: u64,
    pending_updates: u16,
    params: ChannelParams,
    local_keys: ChannelKeys,
    remote_keys: ChannelKeys,

    enquirer: Option<ServiceId>,
}

impl CtlServer for Runtime {}

impl esb::Handler<ServiceBus> for Runtime {
    type Request = Request;
    type Address = ServiceId;
    type Error = Error;

    fn identity(&self) -> ServiceId {
        self.identity.clone()
    }

    fn handle(
        &mut self,
        senders: &mut esb::SenderList<ServiceBus, ServiceId>,
        bus: ServiceBus,
        source: ServiceId,
        request: Request,
    ) -> Result<(), Self::Error> {
        match bus {
            ServiceBus::Msg => self.handle_rpc_msg(senders, source, request),
            ServiceBus::Ctl => self.handle_rpc_ctl(senders, source, request),
            _ => {
                Err(Error::NotSupported(ServiceBus::Bridge, request.get_type()))
            }
        }
    }

    fn handle_err(&mut self, _: esb::Error) -> Result<(), esb::Error> {
        // We do nothing and do not propagate error; it's already being reported
        // with `error!` macro by the controller. If we propagate error here
        // this will make whole daemon panic
        Ok(())
    }
}

impl Runtime {
    fn send_peer(
        &self,
        senders: &mut Senders,
        message: Messages,
    ) -> Result<(), Error> {
        senders.send_to(
            ServiceBus::Msg,
            self.identity(),
            self.peer_service.clone(),
            Request::PeerMessage(message),
        )?;
        Ok(())
    }

    fn handle_rpc_msg(
        &mut self,
        senders: &mut Senders,
        source: ServiceId,
        request: Request,
    ) -> Result<(), Error> {
        match request {
            Request::PeerMessage(Messages::AcceptChannel(accept_channel)) => {
                self.state = ChannelState::Accepted;

                let enquirer = self.enquirer.clone();

                self.channel_accepted(senders, &accept_channel, &source)
                    .map_err(|err| {
                        self.report_failure_to(senders, &enquirer, err)
                    })?;

                // Construct funding output scriptPubkey
                let remote_pk = accept_channel.funding_pubkey;
                let local_pk = self.local_keys.funding_pubkey;
                trace!(
                    "Generating script pubkey from local {} and remote {}",
                    local_pk,
                    remote_pk
                );
                let script_pubkey =
                    PubkeyScript::ln_funding(local_pk, remote_pk);
                trace!("Funding script: {}", script_pubkey);
                if let Some(addr) = script_pubkey.address(self.chain.clone()) {
                    debug!("Funding address: {}", addr);
                } else {
                    error!(
                        "{} {}",
                        "Unable to generate funding address for the current network "
                            .err(),
                        self.chain.err()
                    )
                }

                // Ignoring possible reporting error here: do not want to
                // halt the channel just because the client disconnected
                let _ = self.send_ctl(
                    senders,
                    &enquirer,
                    Request::ChannelFunding(script_pubkey),
                );
            }

            Request::PeerMessage(Messages::FundingSigned(funding_signed)) => {
                // TODO:
                //      1. Get commitment tx
                //      2. Verify signature
                //      3. Save signature/commitment tx
                //      4. Send funding locked request
            }

            Request::PeerMessage(Messages::FundingLocked(funding_locked)) => {
                // TODO:
                //      1. Change the channel state
                //      2. Do something with per-commitment point
            }

            Request::PeerMessage(_) => {
                // Ignore the rest of LN peer messages
            }

            _ => {
                error!(
                    "MSG RPC can be only used for forwarding LNPWP messages"
                );
                return Err(Error::NotSupported(
                    ServiceBus::Msg,
                    request.get_type(),
                ));
            }
        }
        Ok(())
    }

    fn handle_rpc_ctl(
        &mut self,
        senders: &mut Senders,
        source: ServiceId,
        request: Request,
    ) -> Result<(), Error> {
        match request {
            Request::OpenChannelWith(request::CreateChannel {
                channel_req,
                peerd,
                report_to,
            }) => {
                self.peer_service = peerd.clone();
                self.enquirer = report_to.clone();

                if let ServiceId::Peer(ref addr) = peerd {
                    self.remote_peer = Some(addr.clone());
                }

                self.open_channel(senders, &channel_req).map_err(|err| {
                    self.report_failure_to(senders, &report_to, err)
                })?;

                self.send_peer(senders, Messages::OpenChannel(channel_req))?;

                self.state = ChannelState::Proposed;
            }

            Request::AcceptChannelFrom(request::CreateChannel {
                channel_req,
                peerd,
                report_to,
            }) => {
                self.peer_service = peerd.clone();
                self.state = ChannelState::Proposed;

                if let ServiceId::Peer(ref addr) = peerd {
                    self.remote_peer = Some(addr.clone());
                }

                let accept_channel = self
                    .accept_channel(senders, &channel_req, &peerd)
                    .map_err(|err| {
                        self.report_failure_to(senders, &report_to, err)
                    })?;

                self.send_peer(
                    senders,
                    Messages::AcceptChannel(accept_channel),
                )?;

                self.state = ChannelState::Accepted;
            }

            Request::FundChannel(funding_outpoint) => {
                // TODO:
                //      1. Get somehow peerd id
                //      2. Construct commitment tx
                //      3. Sign commitment tx
                //      4. Update channel id
                self.send_peer(
                    senders,
                    Messages::FundingCreated(message::FundingCreated {
                        temporary_channel_id: self.temporary_channel_id,
                        funding_txid: funding_outpoint.txid,
                        funding_output_index: funding_outpoint.vout as u16,
                        signature: Signature::from_compact(&vec![0u8])
                            .expect("This will fail"),
                    }),
                );
            }

            Request::GetInfo => {
                fn bmap<T>(
                    remote_peer: &Option<NodeAddr>,
                    v: &T,
                ) -> BTreeMap<NodeAddr, T>
                where
                    T: Clone,
                {
                    remote_peer
                        .as_ref()
                        .map(|p| bmap! { p.clone() => v.clone() })
                        .unwrap_or_default()
                };

                let info = ChannelInfo {
                    channel_id: self.channel_id,
                    temporary_channel_id: self.temporary_channel_id,
                    state: self.state,
                    local_capacity: self.local_capacity,
                    remote_capacities: bmap(
                        &self.remote_peer,
                        &self.remote_capacity,
                    ),
                    assets: self.local_balances.keys().cloned().collect(),
                    local_balances: self.local_balances.clone(),
                    remote_balances: bmap(
                        &self.remote_peer,
                        &self.remote_balances,
                    ),
                    funding_outpoint: self.funding_outpoint,
                    remote_peers: self
                        .remote_peer
                        .clone()
                        .map(|p| vec![p])
                        .unwrap_or_default(),
                    uptime: SystemTime::now()
                        .duration_since(self.started)
                        .unwrap_or(Duration::from_secs(0)),
                    since: self
                        .started
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap_or(Duration::from_secs(0))
                        .as_secs(),
                    total_updates: self.total_updates,
                    pending_updates: self.pending_updates,
                    params: self.params,
                    local_keys: self.local_keys.clone(),
                    remote_keys: bmap(&self.remote_peer, &self.remote_keys),
                };
                self.send_ctl(senders, source, Request::ChannelInfo(info))?;
            }

            _ => {
                error!("Request is not supported by the CTL interface");
                return Err(Error::NotSupported(
                    ServiceBus::Ctl,
                    request.get_type(),
                ));
            }
        }
        Ok(())
    }
}

impl Runtime {
    pub fn open_channel(
        &mut self,
        senders: &mut Senders,
        channel_req: &message::OpenChannel,
    ) -> Result<(), ChannelNegotiationError> {
        info!(
            "{} remote peer to {} with temp id {:#}",
            "Proposing".promo(),
            "open a channel".promo(),
            channel_req.temporary_channel_id.promoter()
        );
        // Ignoring possible reporting errors here and after: do not want to
        // halt the channel just because the client disconnected
        let enquirer = self.enquirer.clone();
        let _ = self.report_progress_to(
            senders,
            &enquirer,
            format!("Proposing remote peer to open a channel"),
        );

        self.params = ChannelParams::with(&channel_req)?;
        self.local_keys = ChannelKeys::from(channel_req);

        Ok(())
    }

    pub fn accept_channel(
        &mut self,
        senders: &mut Senders,
        channel_req: &message::OpenChannel,
        peerd: &ServiceId,
    ) -> Result<message::AcceptChannel, ChannelNegotiationError> {
        let msg = format!(
            "{} with temp id {:#} from remote peer {}",
            "Accepting channel".promo(),
            channel_req.temporary_channel_id.promoter(),
            peerd.promoter()
        );
        info!("{}", msg);

        // Ignoring possible reporting errors here and after: do not want to
        // halt the channel just because the client disconnected
        let enquirer = self.enquirer.clone();
        let _ = self.report_progress_to(senders, &enquirer, msg);

        self.params = ChannelParams::with(channel_req)?;
        self.remote_keys = ChannelKeys::from(channel_req);

        let dumb_key = self.node_id;
        let accept_channel = message::AcceptChannel {
            temporary_channel_id: channel_req.temporary_channel_id,
            dust_limit_satoshis: channel_req.dust_limit_satoshis,
            max_htlc_value_in_flight_msat: channel_req
                .max_htlc_value_in_flight_msat,
            channel_reserve_satoshis: channel_req.channel_reserve_satoshis,
            htlc_minimum_msat: channel_req.htlc_minimum_msat,
            minimum_depth: 3, // TODO: take from config options
            to_self_delay: channel_req.to_self_delay,
            max_accepted_htlcs: channel_req.max_accepted_htlcs,
            funding_pubkey: dumb_key,
            revocation_basepoint: dumb_key,
            payment_point: dumb_key,
            delayed_payment_basepoint: dumb_key,
            htlc_basepoint: dumb_key,
            first_per_commitment_point: dumb_key,
            shutdown_scriptpubkey: None,
            unknown_tlvs: none!(),
        };

        self.params.updated(&accept_channel, None)?;
        self.local_keys = ChannelKeys::from(&accept_channel);

        let msg = format!(
            "{} channel {:#} from remote peer {}",
            "Accepted".ended(),
            channel_req.temporary_channel_id.ender(),
            peerd.ender()
        );
        info!("{}", msg);
        let _ = self.report_success_to(senders, &enquirer, Some(msg));

        Ok(accept_channel)
    }

    pub fn channel_accepted(
        &mut self,
        senders: &mut Senders,
        accept_channel: &message::AcceptChannel,
        peerd: &ServiceId,
    ) -> Result<(), ChannelNegotiationError> {
        info!(
            "Channel {:#} {} by the remote peer {}",
            accept_channel.temporary_channel_id.ender(),
            "was accepted".ended(),
            peerd.ender()
        );
        // Ignoring possible reporting errors here and after: do not want to
        // halt the channel just because the client disconnected
        let enquirer = self.enquirer.clone();
        let _ = self.report_progress_to(
            senders,
            &enquirer,
            "Channel was accepted by the remote peer",
        );

        let msg = format!(
            "{} returned parameters for the channel {:#}",
            "Verifying".promo(),
            accept_channel.temporary_channel_id.promoter()
        );
        info!("{}", msg);

        // TODO: Add a reasonable min depth bound
        self.params.updated(accept_channel, None)?;
        self.remote_keys = ChannelKeys::from(accept_channel);

        let msg = format!(
            "Channel {:#} is {}",
            accept_channel.temporary_channel_id.ender(),
            "ready for funding".ended()
        );
        info!("{}", msg);
        let _ = self.report_success_to(senders, &enquirer, Some(msg));

        Ok(())
    }
}

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
use std::time::Duration;

use lnpbp::bitcoin::secp256k1::{self};
use lnpbp::bitcoin::{self, OutPoint};
use lnpbp::lnp::{
    message, AssetsBalance, ChannelId, ChannelKeys, ChannelNegotiationError,
    ChannelParams, ChannelState, Messages, NodeAddr, TempChannelId, TypedEnum,
};
use lnpbp::miniscript::{Descriptor, Miniscript, Terminal};
use lnpbp_services::esb::{self, Handler};

use crate::rpc::request::ChannelInfo;
use crate::rpc::{request, Request, ServiceBus};
use crate::{Config, Error, LogStyle, SendTo, Senders, Service, ServiceId};

pub fn run(
    config: Config,
    node_id: secp256k1::PublicKey,
    channel_id: ChannelId,
) -> Result<(), Error> {
    let runtime = Runtime {
        identity: ServiceId::Channel(channel_id),
        node_id,
        channel_id: None,
        temporary_channel_id: channel_id.into(),
        state: default!(),
        local_capacity: 0,
        remote_capacity: 0,
        local_balances: zero!(),
        remote_balances: zero!(),
        funding_outpoint: default!(),
        remote_peer: None,
        uptime: zero!(),
        since: 0,
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
    node_id: secp256k1::PublicKey,

    channel_id: Option<ChannelId>,
    temporary_channel_id: TempChannelId,
    state: ChannelState,
    local_capacity: u64,
    remote_capacity: u64,
    local_balances: AssetsBalance,
    remote_balances: AssetsBalance,
    funding_outpoint: OutPoint,
    remote_peer: Option<NodeAddr>,
    uptime: Duration,
    since: i64,
    total_updates: u64,
    pending_updates: u16,
    params: ChannelParams,
    local_keys: ChannelKeys,
    remote_keys: ChannelKeys,

    enquirer: Option<ServiceId>,
}

impl SendTo for Runtime {}

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
    fn handle_rpc_msg(
        &mut self,
        senders: &mut Senders,
        source: ServiceId,
        request: Request,
    ) -> Result<(), Error> {
        match request {
            Request::PeerMessage(Messages::AcceptChannel(accept_channel)) => {
                let enquirer = self.enquirer.clone();

                self.channel_accepted(senders, &accept_channel, &source)
                    .map_err(|err| {
                        self.report_failure_to(senders, &enquirer, err)
                    })?;

                // Construct funding output scriptPubkey
                // TODO: Move all miniscript constructions to LNP/BP Core
                //       Library
                let lock = Terminal::Multi(
                    2,
                    vec![
                        bitcoin::PublicKey {
                            compressed: false,
                            key: accept_channel.funding_pubkey,
                        },
                        bitcoin::PublicKey {
                            compressed: false,
                            key: self.local_keys.funding_pubkey,
                        },
                    ],
                );
                let ms = Miniscript::from_ast(lock)
                    .expect("miniscript library broken: parse of static miniscript failed");
                let script_pubkey = Descriptor::Wsh(ms).script_pubkey().into();

                // Ignoring possible reporting error here: do not want to
                // halt the channel just because the client disconnected
                let _ = self.send_to(
                    senders,
                    &enquirer,
                    Request::ChannelFunding(script_pubkey),
                );
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
                self.enquirer = report_to.clone();

                if let ServiceId::Peer(ref addr) = peerd {
                    self.remote_peer = Some(addr.clone());
                }

                self.open_channel(senders, &channel_req).map_err(|err| {
                    self.report_failure_to(senders, &report_to, err)
                })?;

                senders.send_to(
                    ServiceBus::Msg,
                    self.identity(),
                    peerd,
                    Request::PeerMessage(Messages::OpenChannel(channel_req)),
                )?;
            }

            Request::AcceptChannelFrom(request::CreateChannel {
                channel_req,
                peerd,
                report_to,
            }) => {
                if let ServiceId::Peer(ref addr) = peerd {
                    self.remote_peer = Some(addr.clone());
                }

                let accept_channel = self
                    .accept_channel(senders, &channel_req, &peerd)
                    .map_err(|err| {
                        self.report_failure_to(senders, &report_to, err)
                    })?;

                senders.send_to(
                    ServiceBus::Msg,
                    self.identity(),
                    peerd,
                    Request::PeerMessage(Messages::AcceptChannel(
                        accept_channel,
                    )),
                )?;
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
                    uptime: self.uptime,
                    since: self.since,
                    total_updates: self.total_updates,
                    pending_updates: self.pending_updates,
                    params: self.params.clone(), // TODO: Remove clone
                    local_keys: self.local_keys.clone(),
                    remote_keys: bmap(&self.remote_peer, &self.remote_keys),
                };
                self.send_to(senders, source, Request::ChannelInfo(info))?;
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
        let msg = format!(
            "Requesting remote peer to {} with temp id {}",
            "open a channel".ended(),
            channel_req.temporary_channel_id.ender()
        );
        info!("{}", msg);
        // Ignoring possible reporting errors here and after: do not want to
        // halt the channel just because the client disconnected
        let enquirer = self.enquirer.clone();
        let _ = self.report_progress_to(senders, &enquirer, msg);

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
            "{} with temp id {} from remote peer {}",
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
            "{} channel {} from remote peer {}",
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
        let msg = format!(
            "Channel {} {} by the remote peer {}",
            accept_channel.temporary_channel_id.promoter(),
            "was accepted".promo(),
            peerd.promoter()
        );
        info!("{}", msg);
        // Ignoring possible reporting errors here and after: do not want to
        // halt the channel just because the client disconnected
        let enquirer = self.enquirer.clone();
        let _ = self.report_progress_to(senders, &enquirer, msg);

        let msg = format!(
            "{} returned parameters for the channel {}",
            "Verifying".promo(),
            accept_channel.temporary_channel_id.promoter()
        );
        info!("{}", msg);
        let _ = self.report_progress_to(senders, &enquirer, msg);

        // TODO: Add a reasonable min depth bound
        self.params.updated(accept_channel, None)?;
        self.remote_keys = ChannelKeys::from(accept_channel);

        let msg = format!(
            "Channel {} is {}",
            accept_channel.temporary_channel_id.promoter(),
            "is ready for funding".promo()
        );
        info!("{}", msg);
        let _ = self.report_success_to(senders, &enquirer, Some(msg));

        Ok(())
    }
}

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

use lnpbp::bitcoin::secp256k1;
use lnpbp::lnp::{message, ChannelId, Messages, TypedEnum};
use lnpbp_services::esb::{self, Handler};

use crate::rpc::{request, Request, ServiceBus};
use crate::{Config, Error, LogStyle, Service, ServiceId};

pub fn run(config: Config, channel_id: ChannelId) -> Result<(), Error> {
    let runtime = Runtime {
        identity: ServiceId::Channel(channel_id),
    };

    Service::run(config, runtime, false)
}

pub struct Runtime {
    identity: ServiceId,
}

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
        _senders: &mut esb::SenderList<ServiceBus, ServiceId>,
        source: ServiceId,
        request: Request,
    ) -> Result<(), Error> {
        match request {
            Request::LnpwpMessage(Messages::AcceptChannel(accept_channel)) => {
                info!(
                    "{} from the remote peer {} with temporary id {}",
                    "Accepting channel".promo(),
                    source.promoter(),
                    accept_channel.temporary_channel_id.promoter()
                );
            }

            Request::LnpwpMessage(_) => {
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
        senders: &mut esb::SenderList<ServiceBus, ServiceId>,
        _source: ServiceId,
        request: Request,
    ) -> Result<(), Error> {
        match request {
            Request::OpenChannelWith(request::ChannelParams {
                channel_req,
                connectiond,
            }) => {
                debug!(
                    "Requesting remote peer to {} with temp id {}",
                    "open a channel".ended(),
                    channel_req.temporary_channel_id.ender()
                );
                senders.send_to(
                    ServiceBus::Msg,
                    self.identity(),
                    connectiond,
                    Request::LnpwpMessage(Messages::OpenChannel(channel_req)),
                )?;
            }
            Request::AcceptChannelFrom(request::ChannelParams {
                channel_req,
                connectiond,
            }) => {
                let dumb_key = secp256k1::PublicKey::from_secret_key(
                    &lnpbp::SECP256K1,
                    &secp256k1::key::ONE_KEY,
                );
                let accept_channel = message::AcceptChannel {
                    temporary_channel_id: channel_req.temporary_channel_id,
                    dust_limit_satoshis: channel_req.dust_limit_satoshis,
                    max_htlc_value_in_flight_msat: channel_req
                        .max_htlc_value_in_flight_msat,
                    channel_reserve_satoshis: channel_req
                        .channel_reserve_satoshis,
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
                senders.send_to(
                    ServiceBus::Msg,
                    self.identity(),
                    connectiond,
                    Request::LnpwpMessage(Messages::AcceptChannel(
                        accept_channel,
                    )),
                )?;
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

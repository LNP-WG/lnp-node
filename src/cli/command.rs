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
use lnpbp::lnp::{
    message, RemoteSocketAddr, TempChannelId, ToNodeAddr,
    LIGHTNING_P2P_DEFAULT_PORT,
};
use lnpbp_services::shell::Exec;

use super::{Command, Runtime};
use crate::rpc::{request, Request};
use crate::{Error, ServiceId};

impl Exec for Command {
    type Runtime = Runtime;
    type Error = Error;

    fn exec(&self, runtime: &mut Self::Runtime) -> Result<(), Self::Error> {
        debug!("Performing {:?}: {}", self, self);
        match self {
            Command::Listen {
                ip_addr,
                port,
                overlay,
            } => {
                let socket =
                    RemoteSocketAddr::with_ip_addr(*overlay, *ip_addr, *port);
                runtime.request(ServiceId::Lnpd, Request::Listen(socket))
            }

            Command::Connect { peer: node_locator } => {
                let peer = node_locator
                    .to_node_addr(LIGHTNING_P2P_DEFAULT_PORT)
                    .expect("Provided node address is invalid");

                runtime.request(ServiceId::Lnpd, Request::ConnectPeer(peer))
            }

            Command::Ping { peer: _ } => {
                unimplemented!()
                /*
                let peer = node_locator
                    .to_node_addr(LIGHTNING_P2P_DEFAULT_PORT)
                    .expect("Provided node address is invalid");

                runtime.request(ServiceId::Lnpd, Request::PingPeer(peer))
                 */
            }

            Command::CreateChannel {
                peer: node_locator,
                satoshis: _,
            } => {
                let peer = node_locator
                    .to_node_addr(LIGHTNING_P2P_DEFAULT_PORT)
                    .expect("Provided node address is invalid");

                let dumb_key = secp256k1::PublicKey::from_secret_key(
                    &lnpbp::SECP256K1,
                    &secp256k1::key::ONE_KEY,
                );

                runtime.request(
                    ServiceId::Lnpd,
                    Request::OpenChannelWith(request::ChannelParams {
                        // TODO: Provide channel configuration from command-line
                        //       arguments and configuration file defaults
                        channel_req: message::OpenChannel {
                            chain_hash: none!(),
                            temporary_channel_id: TempChannelId::random(),
                            funding_satoshis: 0,
                            push_msat: 0,
                            dust_limit_satoshis: 0,
                            max_htlc_value_in_flight_msat: 0,
                            channel_reserve_satoshis: 0,
                            htlc_minimum_msat: 0,
                            feerate_per_kw: 0,
                            to_self_delay: 0,
                            max_accepted_htlcs: 0,
                            funding_pubkey: dumb_key,
                            revocation_basepoint: dumb_key,
                            payment_point: dumb_key,
                            delayed_payment_basepoint: dumb_key,
                            htlc_basepoint: dumb_key,
                            first_per_commitment_point: dumb_key,
                            channel_flags: 0,
                            shutdown_scriptpubkey: None,
                            unknown_tlvs: none!(),
                        },
                        peerd: ServiceId::Peer(peer),
                    }),
                )
            }

            _ => unimplemented!(),
        }
    }
}

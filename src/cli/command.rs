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

use amplify::Wrapper;
use lnpbp::bitcoin::secp256k1;
use lnpbp::lnp::{application::TempChannelId, message};
use lnpbp_services::shell::Exec;

use super::{Command, Runtime};
use crate::rpc::{request, Request};
use crate::{DaemonId, Error};

impl Exec for Command {
    type Runtime = Runtime;
    type Error = Error;

    fn exec(&self, runtime: &mut Self::Runtime) -> Result<(), Self::Error> {
        debug!("Performing {:?}: {}", self, self);
        match self {
            Command::Ping => runtime.request(DaemonId::Lnpd, Request::PingPeer),
            Command::CreateChannel { node_addr } => {
                let dumb_key = secp256k1::PublicKey::from_secret_key(
                    &lnpbp::SECP256K1,
                    &secp256k1::key::ONE_KEY,
                );
                runtime.request(
                    DaemonId::Lnpd,
                    Request::CreateChannel(request::CreateChannel {
                        channel_req: message::OpenChannel {
                            chain_hash: none!(),
                            temporary_channel_id: TempChannelId::from_inner(
                                [0u8; 32],
                            ),
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
                        connectiond: DaemonId::Connection(
                            node_addr.to_string(),
                        ),
                    }),
                )
            }
        }
    }
}

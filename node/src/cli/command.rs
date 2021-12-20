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

use std::str::FromStr;

use internet2::{NodeAddr, RemoteSocketAddr, ToNodeAddr, ToRemoteNodeAddr};
use lnp::p2p::legacy::{ChannelId, LNP2P_LEGACY_PORT};
use microservices::shell::Exec;
#[cfg(feature = "rgb")]
use rgb::Consignment;
#[cfg(feature = "rgb")]
use rgb_node::util::file::ReadWrite;

use super::Command;
use crate::i9n::rpc::{self as request, RpcMsg};
use crate::i9n::Client;
use crate::{Error, LogStyle, ServiceId};

impl Exec for Command {
    type Client = Client;
    type Error = Error;

    fn exec(self, runtime: &mut Self::Client) -> Result<(), Self::Error> {
        debug!("Performing {:?}: {}", self, self);
        match self {
            Command::Info { subject } => {
                if let Some(subj) = subject {
                    if let Ok(node_addr) = NodeAddr::from_str(&subj) {
                        runtime.request(ServiceId::Peer(node_addr), RpcMsg::GetInfo)?;
                    } else if let Ok(channel_id) = ChannelId::from_str(&subj) {
                        runtime.request(ServiceId::Channel(channel_id), RpcMsg::GetInfo)?;
                    } else {
                        let err = format!(
                            "{}",
                            "Subject parameter must be either remote node address or channel id \
                             represented by a hex string"
                                .err()
                        );
                        return Err(Error::Other(err));
                    }
                } else {
                    runtime.request(ServiceId::LnpBroker, RpcMsg::GetInfo)?;
                }
                match runtime.response()? {
                    RpcMsg::NodeInfo(info) => println!("{}", info),
                    RpcMsg::PeerInfo(info) => println!("{}", info),
                    RpcMsg::ChannelInfo(info) => println!("{}", info),
                    _ => {
                        Err(Error::Other(format!("{}", "Server returned unrecognizable response")))?
                    }
                }
            }

            Command::Peers => {
                runtime.request(ServiceId::LnpBroker, RpcMsg::ListPeers)?;
                runtime.report_response()?;
            }

            Command::Channels => {
                runtime.request(ServiceId::LnpBroker, RpcMsg::ListChannels)?;
                runtime.report_response()?;
            }

            Command::Funds => {
                runtime.request(ServiceId::LnpBroker, RpcMsg::ListFunds)?;
                runtime.report_response()?;
            }

            Command::Listen { ip_addr, port, overlay } => {
                let socket = RemoteSocketAddr::with_ip_addr(overlay, ip_addr, port);
                runtime.request(ServiceId::LnpBroker, RpcMsg::Listen(socket))?;
                runtime.report_progress()?;
            }

            Command::Connect { peer: node_locator } => {
                let peer = node_locator
                    .to_remote_node_addr(LNP2P_LEGACY_PORT)
                    .expect("Provided node address is invalid");

                runtime.request(ServiceId::LnpBroker, RpcMsg::ConnectPeer(peer))?;
                runtime.report_progress()?;
            }

            Command::Ping { peer } => {
                let node_addr =
                    peer.to_node_addr(LNP2P_LEGACY_PORT).expect("node address is invalid");

                runtime.request(ServiceId::Peer(node_addr), RpcMsg::PingPeer)?;
            }

            Command::Open {
                peer,
                funding_sat,
                push_msat,
                fee_rate,
                announce_channel,
                channel_type,
                dust_limit,
                to_self_delay,
                htlc_max_count,
                htlc_min_value,
                htlc_max_total_value,
                channel_reserve,
            } => {
                let node_addr =
                    peer.to_node_addr(LNP2P_LEGACY_PORT).expect("node address is invalid");

                runtime.request(
                    ServiceId::LnpBroker,
                    RpcMsg::CreateChannel(request::CreateChannel {
                        funding_sat,
                        push_msat: push_msat.unwrap_or_default(),
                        fee_rate,
                        announce_channel,
                        channel_type,
                        dust_limit,
                        to_self_delay,
                        htlc_max_count,
                        htlc_min_value,
                        htlc_max_total_value,
                        remote_peer: node_addr,
                        report_to: Some(runtime.identity),
                        channel_reserve,
                    }),
                )?;
                runtime.report_progress()?;
            }

            #[cfg(feature = "rgb")]
            Command::Transfer { channel, amount, asset } => {
                runtime.request(
                    channel.clone().into(),
                    RpcMsg::Transfer(request::Transfer {
                        channeld: channel.clone().into(),
                        amount,
                        asset: asset.map(|id| id.into()),
                    }),
                )?;
                runtime.report_progress()?;
            }

            #[cfg(feature = "rgb")]
            Command::Refill { channel, consignment, outpoint, blinding_factor } => {
                trace!("Reading consignment from file {:?}", &consignment);
                let consignment = Consignment::read_file(consignment.clone()).map_err(|err| {
                    Error::Other(format!("Error in consignment encoding: {}", err))
                })?;
                trace!("Outpoint parsed as {}", outpoint);

                runtime.request(
                    channel.clone().into(),
                    RpcMsg::RefillChannel(rprequestc::RefillChannel {
                        consignment,
                        outpoint,
                        blinding: blinding_factor,
                    }),
                )?;
                runtime.report_progress()?;
            }

            _ => todo!(),
        }
        Ok(())
    }
}

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
use lnp_rpc::{self, Client, CreateChannel, Error, PayInvoice, RpcMsg, ServiceId};
use microservices::shell::Exec;

use crate::opts::Command;

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
                        return Err(Error::Other(s!("Subject parameter must be either remote \
                                                    node address or channel id represented by \
                                                    a hex string")));
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
                    RpcMsg::CreateChannel(CreateChannel {
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
                        report_to: Some(runtime.identity()),
                        channel_reserve,
                    }),
                )?;
                runtime.report_progress()?;
            }
            Command::Invoice { .. } => todo!("Implement invoice generation"),

            Command::Pay { invoice, channel: channel_id, amount_msat } => {
                runtime.request(
                    ServiceId::Router,
                    RpcMsg::PayInvoice(PayInvoice { invoice, channel_id, amount_msat }),
                )?;
                runtime.report_progress()?;
            }
        }
        Ok(())
    }
}

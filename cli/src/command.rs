// LNP Node: node running lightning network protocol and generalized lightning
// channels.
// Written in 2020-2022 by
//     Dr. Maxim Orlovsky <orlovsky@lnp-bp.org>
//
// To the extent possible under law, the author(s) have dedicated all
// copyright and related and neighboring rights to this software to
// the public domain worldwide. This software is distributed without
// any warranty.
//
// You should have received a copy of the MIT License along with this software.
// If not, see <https://opensource.org/licenses/MIT>.

use std::net::SocketAddr;
use std::str::FromStr;

use internet2::addr::NodeAddr;
use lnp::p2p;
use lnp::p2p::bifrost::LNP2P_BIFROST_PORT;
use lnp::p2p::bolt::{ChannelId, LNP2P_LEGACY_PORT};
use lnp_rpc::{self, Client, ConnectInfo, CreateChannel, Error, PayInvoice, RpcMsg, ServiceId};
use microservices::shell::Exec;

use crate::{Command, Opts};

impl Command {
    pub fn action_string(&self) -> String {
        match self {
            Command::Listen { .. } => s!("Binding to port"),
            Command::Connect { .. } => s!("Connecting to remore peer"),
            Command::Ping { .. } => s!("Pinging peer"),
            Command::Info { .. } => s!("Getting info"),
            Command::Funds => s!("Retrieving information about funds"),
            Command::Peers => s!("Retrieving information about peers"),
            Command::Channels => s!("Retrieving information about channels"),
            Command::Open { .. } => s!("Opening channel"),
            Command::Invoice { .. } => s!("Creating invoice"),
            Command::Pay { .. } => s!("Paying invoice"),
        }
    }
}

impl Exec for Opts {
    type Client = Client;
    type Error = Error;

    fn exec(self, runtime: &mut Self::Client) -> Result<(), Self::Error> {
        println!("{}...", self.command.action_string());
        match self.command {
            Command::Info { subject } => {
                if let Some(subj) = subject {
                    if let Ok(node_addr) = NodeAddr::from_str(&subj) {
                        runtime.request(ServiceId::PeerBolt(node_addr), RpcMsg::GetInfo)?;
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
                        return Err(Error::Other(
                            "Server returned unrecognizable response".to_string(),
                        ))
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

            Command::Listen { ip_addr, port } => {
                let socket = SocketAddr::new(ip_addr, port);
                runtime.request(ServiceId::LnpBroker, RpcMsg::Listen(socket))?;
                runtime.report_progress()?;
            }

            Command::Connect { peer, bolt: true, bifrost: false } => {
                let addr = peer.node_addr(LNP2P_LEGACY_PORT);

                runtime.request(
                    ServiceId::LnpBroker,
                    RpcMsg::ConnectPeer(ConnectInfo { addr, protocol: p2p::Protocol::Bolt }),
                )?;
                runtime.report_progress()?;
            }

            Command::Connect { peer, bolt: false, bifrost: true } => {
                let addr = peer.node_addr(LNP2P_BIFROST_PORT);

                runtime.request(
                    ServiceId::LnpBroker,
                    RpcMsg::ConnectPeer(ConnectInfo { addr, protocol: p2p::Protocol::Bifrost }),
                )?;
                runtime.report_progress()?;
            }

            Command::Connect { peer: _, bolt: _, bifrost: _ } => unreachable!(),

            Command::Ping { peer } => {
                let node_addr = peer.node_addr(LNP2P_LEGACY_PORT);

                runtime.request(ServiceId::PeerBolt(node_addr), RpcMsg::PingPeer)?;
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
                let node_addr = peer.node_addr(LNP2P_LEGACY_PORT);

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

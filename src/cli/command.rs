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

use std::convert::TryFrom;
use std::str::FromStr;

use internet2::{NodeAddr, RemoteSocketAddr, ToNodeAddr};
use lnp::p2p::legacy::{ChannelId, OpenChannel, LNP2P_LEGACY_PORT};
use microservices::shell::Exec;

#[cfg(feature = "rgb")]
use rgb::Consignment;
#[cfg(feature = "rgb")]
use rgb_node::util::file::ReadWrite;

use super::Command;
use crate::rpc::{request, Client, Request};
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
                        runtime.request(
                            ServiceId::Peer(node_addr),
                            Request::GetInfo,
                        )?;
                    } else if let Ok(channel_id) = ChannelId::from_str(&subj) {
                        runtime.request(
                            ServiceId::Channel(channel_id),
                            Request::GetInfo,
                        )?;
                    } else {
                        let err = format!(
                            "{}",
                            "Subject parameter must be either remote node \
                            address or channel id represented by a hex string"
                                .err()
                        );
                        return Err(Error::Other(err));
                    }
                } else {
                    runtime.request(ServiceId::Lnpd, Request::GetInfo)?;
                }
                match runtime.response()? {
                    Request::NodeInfo(info) => println!("{}", info),
                    Request::PeerInfo(info) => println!("{}", info),
                    Request::ChannelInfo(info) => println!("{}", info),
                    _ => Err(Error::Other(format!(
                        "{}",
                        "Server returned unrecognizable response"
                    )))?,
                }
            }

            Command::Peers => {
                runtime.request(ServiceId::Lnpd, Request::ListPeers)?;
                runtime.report_response()?;
            }

            Command::Channels => {
                runtime.request(ServiceId::Lnpd, Request::ListChannels)?;
                runtime.report_response()?;
            }

            Command::Listen {
                ip_addr,
                port,
                overlay,
            } => {
                let socket =
                    RemoteSocketAddr::with_ip_addr(overlay, ip_addr, port);
                runtime.request(ServiceId::Lnpd, Request::Listen(socket))?;
                runtime.report_progress()?;
            }

            Command::Connect { peer: node_locator } => {
                let peer = node_locator
                    .to_node_addr(LNP2P_LEGACY_PORT)
                    .expect("Provided node address is invalid");

                runtime.request(ServiceId::Lnpd, Request::ConnectPeer(peer))?;
                runtime.report_progress()?;
            }

            Command::Ping { peer } => {
                let node_addr = peer
                    .to_node_addr(LNP2P_LEGACY_PORT)
                    .expect("Provided node address is invalid");

                runtime
                    .request(ServiceId::Peer(node_addr), Request::PingPeer)?;
            }

            Command::Propose {
                peer,
                funding_satoshis,
            } => {
                let node_addr = peer
                    .to_node_addr(LNP2P_LEGACY_PORT)
                    .expect("Provided node address is invalid");

                runtime.request(
                    ServiceId::Lnpd,
                    Request::OpenChannelWith(request::CreateChannel {
                        channel_req: OpenChannel {
                            funding_satoshis,
                            // The rest of parameters will be filled in by the
                            // daemon
                            ..dumb!()
                        },
                        peerd: ServiceId::Peer(node_addr),
                        report_to: Some(runtime.identity()),
                    }),
                )?;
                runtime.report_progress()?;
                match runtime.response()? {
                    Request::ChannelFunding(pubkey_script) => {
                        let address =
                            bitcoin::Network::try_from(runtime.chain())
                                .ok()
                                .and_then(|network| {
                                    pubkey_script.address(network)
                                });
                        match address {
                            None => {
                                eprintln!(
                                    "{}", 
                                    "Can't generate funding address for a given network".err()
                                );
                                println!(
                                    "{}\nAssembly: {}\nHex: {:x}",
                                    "Please transfer channel funding to an output \
                                     with the following raw `scriptPubkey`"
                                        .progress(),
                                    pubkey_script,
                                    pubkey_script,
                                );
                            }
                            Some(address) => {
                                println!(
                                    "{} {}",
                                    "Please transfer channel funding to "
                                        .progress(),
                                    address.ended()
                                );
                            }
                        }
                    }
                    other => {
                        eprintln!(
                            "{} {} {}",
                            "Unexpected server response".err(),
                            other,
                            "while waiting for channel funding information"
                                .err()
                        );
                    }
                }
            }

            Command::Fund {
                channel,
                funding_outpoint,
            } => {
                runtime.request(
                    channel.clone().into(),
                    Request::FundChannel(funding_outpoint),
                )?;
                runtime.report_progress()?;
            }

            #[cfg(feature = "rgb")]
            Command::Transfer {
                channel,
                amount,
                asset,
            } => {
                runtime.request(
                    channel.clone().into(),
                    Request::Transfer(request::Transfer {
                        channeld: channel.clone().into(),
                        amount,
                        asset: asset.map(|id| id.into()),
                    }),
                )?;
                runtime.report_progress()?;
            }

            #[cfg(feature = "rgb")]
            Command::Refill {
                channel,
                consignment,
                outpoint,
                blinding_factor,
            } => {
                trace!("Reading consignment from file {:?}", &consignment);
                let consignment = Consignment::read_file(consignment.clone())
                    .map_err(|err| {
                    Error::Other(format!(
                        "Error in consignment encoding: {}",
                        err
                    ))
                })?;
                trace!("Outpoint parsed as {}", outpoint);

                runtime.request(
                    channel.clone().into(),
                    Request::RefillChannel(request::RefillChannel {
                        consignment,
                        outpoint,
                        blinding: blinding_factor,
                    }),
                )?;
                runtime.report_progress()?;
            }

            _ => unimplemented!(),
        }
        Ok(())
    }
}

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

use lnpbp::lnp::{
    message, RemoteSocketAddr, ToNodeAddr, LIGHTNING_P2P_DEFAULT_PORT,
};
use lnpbp_services::shell::Exec;

use super::{Command, Runtime};
use crate::rpc::{request, Request};
use crate::{Error, LogStyle, ServiceId};

impl Exec for Command {
    type Runtime = Runtime;
    type Error = Error;

    fn exec(&self, runtime: &mut Self::Runtime) -> Result<(), Self::Error> {
        debug!("Performing {:?}: {}", self, self);
        match self {
            Command::Info => {
                runtime.request(ServiceId::Lnpd, Request::GetInfo)?;
                runtime.report_response()?;
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
                    RemoteSocketAddr::with_ip_addr(*overlay, *ip_addr, *port);
                runtime.request(ServiceId::Lnpd, Request::Listen(socket))?;
                runtime.report_progress()?;
            }

            Command::Connect { peer: node_locator } => {
                let peer = node_locator
                    .to_node_addr(LIGHTNING_P2P_DEFAULT_PORT)
                    .expect("Provided node address is invalid");

                runtime.request(ServiceId::Lnpd, Request::ConnectPeer(peer))?;
                runtime.report_progress()?;
            }

            Command::Ping { peer } => {
                let node_addr = peer
                    .to_node_addr(LIGHTNING_P2P_DEFAULT_PORT)
                    .expect("Provided node address is invalid");

                runtime
                    .request(ServiceId::Peer(node_addr), Request::PingPeer)?;
            }

            Command::Propose {
                peer,
                funding_satoshis,
            } => {
                let node_addr = peer
                    .to_node_addr(LIGHTNING_P2P_DEFAULT_PORT)
                    .expect("Provided node address is invalid");

                runtime.request(
                    ServiceId::Lnpd,
                    Request::OpenChannelWith(request::CreateChannel {
                        channel_req: message::OpenChannel {
                            funding_satoshis: *funding_satoshis,
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
                        let address = pubkey_script.address(runtime.chain());
                        match address {
                            None => {
                                eprintln!(
                                    "{}", 
                                    "Can't generate funding address for a given network".err()
                                );
                                println!(
                                    "{}\nAssembly: {}\nHex: {:x}",
                                    "Please transfer channel funding to an output with the following raw `scriptPubkey`"
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

            _ => unimplemented!(),
        }
        Ok(())
    }
}

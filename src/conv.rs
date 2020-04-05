// Lightning network protocol (LNP) daemon suite
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

use std::{
    str::FromStr,
    net::{IpAddr, SocketAddr}
};
use lnpbp::bitcoin::secp256k1::PublicKey;
use super::constants::LNP2P_PORT;

pub fn conv_pubkey(pubkey_str: &str) -> Result<PublicKey, String> {
    match PublicKey::from_str(pubkey_str) {
        Ok(pubkey) => Ok(pubkey),
        Err(_) => Err(String::from("The provided string does not correspond to a pubkey"))
    }
}

pub fn conv_ip_port(addr_str: &str) -> Result<SocketAddr, String> {
    let mut components = addr_str.split(':');
    let ip_str = components.next()
        .ok_or("No IP address specified")?;
    let ip = ip_str.parse()
        .map_err(|_| "The provided IP address is invalid")?;
    let port: u16 = match components.next() {
        None => LNP2P_PORT,
        Some(port_str) => port_str.parse()
            .map_err(|_| format!("The provided value for port ({}) is invalid", port_str))?,
    };
    if let Some(_) = components.next() {
        Err(String::from("The provided string neither represents IPv4 nor IPv6 address"))?;
    }
    Ok(SocketAddr::new(ip, port))
}

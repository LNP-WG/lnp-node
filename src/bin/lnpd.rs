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

#![recursion_limit = "256"]
// Coding conventions
#![deny(
    non_upper_case_globals,
    non_camel_case_types,
    non_snake_case,
    unused_mut,
    unused_imports,
    dead_code,
    missing_docs
)]

//! Main executable for lnpd: lightning node management microservice.

#[macro_use]
extern crate log;

use std::io::Write;
use std::net::{IpAddr, Ipv4Addr};
use std::path::{Path, PathBuf};

use bitcoin::XpubIdentifier;
use clap::Parser;
use internet2::addr::LocalNode;
use lnp_node::lnpd::{self, read_node_key_file, Command, Opts};
use lnp_node::{Config, Error};
use lnp_rpc::ListenAddr;
use microservices::cli::LogStyle;
use strict_encoding::{StrictDecode, StrictEncode};
use wallet::hd::DerivationAccount;

fn main() -> Result<(), Error> {
    println!("lnpd: lightning node management microservice");

    let mut opts = Opts::parse();
    trace!("Command-line arguments: {:?}", opts);
    opts.process();
    trace!("Processed arguments: {:?}", opts);

    let config: Config = opts.clone().into();
    trace!("Daemon configuration: {:?}", config);
    debug!("MSG socket {}", config.msg_endpoint);
    debug!("CTL socket {}", config.ctl_endpoint);
    debug!("RPC socket {}", config.rpc_endpoint);

    /*
    use self::internal::ResultExt;
    let (config_from_file, _) =
        internal::Config::custom_args_and_optional_files(std::iter::empty::<
            &str,
        >())
        .unwrap_or_exit();
     */

    let key_file = PathBuf::from(opts.key_opts.key_file);
    let listen_addr = opts.listen.map(|maybe_ip: Option<IpAddr>| {
        let ip = maybe_ip.unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED));
        match opts.bifrost {
            true => ListenAddr::bifrost(ip, opts.port),
            false => ListenAddr::bolt(ip, opts.port),
        }
    });

    if let Some(command) = opts.command {
        match command {
            Command::Init => init(&config, &key_file)?,
        }
    }

    debug!("Starting runtime ...");
    lnpd::run(config, key_file, listen_addr).expect("running lnpd runtime");

    unreachable!()
}

fn init(config: &Config, key_path: &Path) -> Result<(), Error> {
    use std::fs;
    use std::process::exit;
    use std::str::FromStr;

    use bitcoin::secp256k1::Secp256k1;
    use bitcoin::util::bip32::{ChildNumber, DerivationPath, ExtendedPrivKey};
    use lnp_node::lnpd::funding::FundingWallet;
    use lnp_node::{LNP_NODE_FUNDING_WALLET, LNP_NODE_MASTER_KEY_FILE};
    use miniscript::descriptor::{Descriptor, Wpkh};
    use wallet::hd::TerminalStep;
    use wallet::psbt::sign::MemorySigningAccount;

    let secp = Secp256k1::new();
    let chain_index = config.chain.chain_params().is_testnet as u16;

    println!("\n{}", "Initializing node data".progress());

    if !config.data_dir.exists() {
        println!("Data directory '{}' ... {}", config.data_dir.display(), "creating".action());
        fs::create_dir_all(&config.data_dir)?;
    } else {
        println!("Data directory '{}' ... {}", config.data_dir.display(), "found".progress());
    }

    let mut wallet_path = config.data_dir.clone();
    wallet_path.push(LNP_NODE_MASTER_KEY_FILE);
    let signing_account = if !wallet_path.exists() {
        println!("Signing account '{}' ... {}", LNP_NODE_MASTER_KEY_FILE, "creating".action());
        let xpriv = rpassword::read_password_from_tty(Some("Please enter your master xpriv: "))?;
        let xpriv = ExtendedPrivKey::from_str(&xpriv)?;
        let derivation = DerivationPath::from_str("m/9735h").expect("hardcoded derivation path");
        let xpriv_account = xpriv.derive_priv(&secp, &derivation)?;
        let xpubid = xpriv.identifier(&secp);

        let mut file = fs::File::create(wallet_path)?;
        xpubid.strict_encode(&mut file)?;
        derivation.strict_encode(&mut file)?;
        xpriv_account.strict_encode(&mut file)?;

        MemorySigningAccount::with(&secp, xpubid, derivation, xpriv_account)
    } else {
        println!("Signing account '{}' ... {}", LNP_NODE_MASTER_KEY_FILE, "found".progress());
        let mut file = fs::File::open(wallet_path)?;
        let master_id = XpubIdentifier::strict_decode(&mut file)?;
        let derivation = DerivationPath::strict_decode(&mut file)?;
        let account_xpriv = ExtendedPrivKey::strict_decode(&mut file)?;
        MemorySigningAccount::with(&secp, master_id, derivation, account_xpriv)
    };
    println!(
        "Signing account: {}",
        format!(
            "m=[{}]/{}=[{}]",
            signing_account.master_fingerprint(),
            signing_account.derivation().to_string().trim_start_matches("m/"),
            signing_account.account_xpub(),
        )
        .announce()
    );

    let mut wallet_path = config.data_dir.clone();
    wallet_path.push(LNP_NODE_FUNDING_WALLET);
    let funding_wallet = if !wallet_path.exists() {
        println!("Funding wallet '{}' ... {}", LNP_NODE_FUNDING_WALLET, "creating".action());
        let account_path = &[chain_index, 2][..];
        let node_xpriv = signing_account.account_xpriv();
        let account_xpriv = node_xpriv.derive_priv(
            &secp,
            &account_path
                .iter()
                .copied()
                .map(u32::from)
                .map(ChildNumber::from_hardened_idx)
                .collect::<Result<Vec<_>, _>>()
                .expect("hardcoded derivation indexes"),
        )?;
        let account = DerivationAccount::with(
            &secp,
            *signing_account.master_id(),
            account_xpriv,
            account_path,
            vec![TerminalStep::range(0u16, 1u16), TerminalStep::Wildcard],
        );
        let descriptor = Descriptor::Wpkh(Wpkh::new(account)?);
        FundingWallet::new(&config.chain, wallet_path, descriptor, &config.electrum_url)?
    } else {
        println!("Funding wallet '{}' ... {}", LNP_NODE_FUNDING_WALLET, "found".progress());
        FundingWallet::with(&config.chain, wallet_path, &config.electrum_url)?
    };
    println!("Funding wallet: {}", funding_wallet.descriptor().announce());

    let node_key = if !key_path.exists() {
        println!("Node key file '{}' ... {}", key_path.display(), "creating".action());

        let derivation_path = DerivationPath::from_str(&format!("m/9735h/{}h/0h", chain_index))
            .expect("hardcoded derivation path");
        let node_secret = signing_account.derive_seckey(&secp, &derivation_path);
        let local_node = LocalNode::with(&secp, node_secret);
        let mut key_file = fs::File::create(key_path).unwrap_or_else(|_| {
            panic!(
                "Unable to create node key file '{}'; please check that the path exists",
                key_path.display()
            )
        });
        key_file.write_all(&node_secret[..]).expect("Unable to save generated node key file");
        local_node
    } else {
        println!("Node key file '{}' ... {}", key_path.display(), "found".action());
        read_node_key_file(key_path)
    };
    println!("Node key: {}", node_key.node_id().announce());

    println!("{}", "Node initialization complete\n".ended());

    exit(0);
}

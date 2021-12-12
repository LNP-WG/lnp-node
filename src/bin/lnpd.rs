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

use clap::Parser;
use lnp_node::lnpd::{self, Command, Opts};
use lnp_node::{Config, Error, LogStyle};

fn main() -> Result<(), Error> {
    println!("lnpd: lightning node management microservice");

    let mut opts = Opts::parse();
    trace!("Command-line arguments: {:?}", &opts);
    opts.process();
    trace!("Processed arguments: {:?}", &opts);

    let config: Config = opts.shared.clone().into();
    trace!("Daemon configuration: {:?}", &config);
    debug!("MSG RPC socket {}", &config.msg_endpoint);
    debug!("CTL RPC socket {}", &config.ctl_endpoint);

    if let Some(command) = opts.command {
        match command {
            Command::Init => init(&config)?,
        }
    }

    let node_id = opts.key_opts.local_node().node_id();
    info!("{}: {}", "Local node id".ended(), node_id.addr());

    /*
    use self::internal::ResultExt;
    let (config_from_file, _) =
        internal::Config::custom_args_and_optional_files(std::iter::empty::<
            &str,
        >())
        .unwrap_or_exit();
     */

    debug!("Starting runtime ...");
    lnpd::run(config, node_id, opts.threaded_daemons).expect("running lnpd runtime");

    unreachable!()
}

fn init(config: &Config) -> Result<(), Error> {
    use std::fs;
    use std::process::exit;
    use std::str::FromStr;

    use bitcoin::secp256k1::Secp256k1;
    use bitcoin::util::bip32::{ChildNumber, DerivationPath, ExtendedPrivKey};
    use bitcoin_hd::{TerminalStep, TrackingAccount};
    use lnp_node::lnpd::funding_wallet::FundingWallet;
    use lnp_node::opts::{LNP_NODE_FUNDING_WALLET, LNP_NODE_MASTER_WALLET};
    use miniscript::descriptor::{Descriptor, Wpkh};
    use psbt::sign::MemorySigningAccount;

    let secp = Secp256k1::new();

    println!("\n{}", "Initializing node data".progress());

    if !config.data_dir.exists() {
        println!("Data directory '{}' ... {}", config.data_dir.display(), "creating".action());
        fs::create_dir_all(&config.data_dir)?;
    } else {
        println!("Data directory '{}' ... {}", config.data_dir.display(), "found".progress());
    }

    let mut wallet_path = config.data_dir.clone();
    wallet_path.push(LNP_NODE_MASTER_WALLET);
    let signing_account = if !wallet_path.exists() {
        println!("Signing account '{}' ... {}", LNP_NODE_MASTER_WALLET, "creating".action());
        let xpriv = rpassword::read_password_from_tty(Some("Please enter your master xpriv: "))?;
        let xpriv = ExtendedPrivKey::from_str(&xpriv)?;
        let derivation = DerivationPath::from_str("m/10046h").expect("hardcoded derivation path");
        let xpriv_account = xpriv.derive_priv(&secp, &derivation)?;
        let fingerprint = xpriv.identifier(&secp);
        let signing_account =
            MemorySigningAccount::with(&secp, fingerprint, derivation, xpriv_account);
        let file = fs::File::create(wallet_path)?;
        signing_account.write(file)?;
        signing_account
    } else {
        println!("Signing account '{}' ... {}", LNP_NODE_MASTER_WALLET, "found".progress());
        MemorySigningAccount::read(&secp, fs::File::open(wallet_path)?)?
    };
    println!(
        "Signing account: {}",
        format!(
            "m=[{}]/10046h=[{}]",
            signing_account.master_fingerprint(),
            signing_account.account_xpub(),
        )
        .promo()
    );

    let mut wallet_path = config.data_dir.clone();
    wallet_path.push(LNP_NODE_FUNDING_WALLET);
    let funding_wallet = if !wallet_path.exists() {
        println!("Funding wallet '{}' ... {}", LNP_NODE_FUNDING_WALLET, "creating".action());
        let account_path = &[10046_u16, 0, 2][..];
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
        let account = TrackingAccount::with(
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
    println!("Funding wallet: {}", funding_wallet.descriptor().promo());

    println!("{}", "Node initialization complete\n".ended());

    exit(0);
}

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

use std::collections::{BTreeMap, HashSet};
use std::convert::TryInto;
use std::io::Seek;
use std::path::Path;
use std::{fs, io};

use amplify::{IoError, Slice32, ToYamlString};
use bitcoin::secp256k1::{self, Secp256k1};
use bitcoin::{Address, Amount, Script};
use bitcoin_hd::{
    DeriveError, DescriptorDerive, SegmentIndexes, TrackingAccount,
    UnhardenedIndex,
};
use bitcoin_onchain::blockchain::Utxo;
use bitcoin_onchain::{ResolveUtxo, UtxoResolverError};
use electrum_client::Client as ElectrumClient;
use lnpbp::chain::{Chain, ConversionImpossibleError};
use miniscript::Descriptor;
use strict_encoding::{StrictDecode, StrictEncode};
use wallet::address::AddressCompat;

/// Errors working with funding wallet
#[derive(Debug, Display, From)]
#[display(doc_comments)]
#[non_exhaustive]
pub enum Error {
    /// Error accessing funding wallet file
    #[from(io::Error)]
    Io(IoError),

    /// Error reading or writing funding wallet data
    #[from]
    StrictEncoding(strict_encoding::Error),

    /// Error resolving funding wallet transactions with Electrum server
    #[from]
    Electrum(electrum_client::Error),

    /// Error resolving funding wallet transactions
    #[from]
    Resolver(UtxoResolverError),

    /// Funding wallet uses custom descriptor which can't be represented as a
    /// valid bitcoin addresses, making channel funding impossible
    NoAddressRepresentation,

    /// Chain is not supported for funding
    #[from(ConversionImpossibleError)]
    ChainNotSupported,

    /// chain network mismatches funding wallet network
    ChainMismatch,

    /// Unable to derive an address for the descriptor; potentially funding
    /// wallet descriptor is incorrect
    #[from]
    Derivation(DeriveError),

    /// Wallet has run out of funding addresses
    OutOfIndexes,
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(err) => Some(err),
            Error::StrictEncoding(err) => Some(err),
            Error::Electrum(err) => Some(err),
            Error::NoAddressRepresentation => None,
            Error::ChainNotSupported => None,
            Error::Resolver(err) => Some(err),
            Error::Derivation(err) => Some(err),
            Error::OutOfIndexes => None,
            Error::ChainMismatch => None,
        }
    }
}

#[derive(
    Clone,
    Ord,
    PartialOrd,
    Eq,
    PartialEq,
    Hash,
    Debug,
    StrictEncode,
    StrictDecode,
)]
#[cfg_attr(
    feature = "serde",
    derive(Display, Serialize, Deserialize),
    serde(crate = "serde_crate"),
    display(WalletData::to_yaml_string)
)]
pub struct WalletData {
    pub descriptor: Descriptor<TrackingAccount>,
    pub last_normal_index: UnhardenedIndex,
    pub last_change_index: UnhardenedIndex,
    pub last_rgb_index: BTreeMap<Slice32, UnhardenedIndex>,
}

#[cfg(feature = "serde")]
impl ToYamlString for WalletData {}

#[derive(Getters)]
pub struct FundingWallet {
    #[getter(skip)]
    secp: Secp256k1<secp256k1::All>,
    network: bitcoin::Network,
    #[getter(skip)]
    resolver: ElectrumClient,
    #[getter(skip)]
    wallet_file: fs::File,
    wallet_data: WalletData,
}

impl FundingWallet {
    pub fn new(
        chain: &Chain,
        wallet_path: impl AsRef<Path>,
        wallet_data: WalletData,
        electrum_url: &str,
    ) -> Result<FundingWallet, Error> {
        let wallet_file = fs::File::create(wallet_path)?;
        wallet_data.strict_encode(&wallet_file)?;
        Ok(FundingWallet {
            secp: Secp256k1::new(),
            network: chain.try_into()?,
            resolver: ElectrumClient::new(electrum_url)?,
            wallet_file,
            wallet_data,
        })
    }

    pub fn with(
        chain: &Chain,
        wallet_path: impl AsRef<Path>,
        electrum_url: &str,
    ) -> Result<FundingWallet, Error> {
        let wallet_file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(false)
            .open(wallet_path)?;
        let wallet_data = WalletData::strict_decode(&wallet_file)?;

        let network = chain.try_into()?;
        if wallet_data.descriptor.network()? != network {
            return Err(Error::ChainMismatch);
        }

        Ok(FundingWallet {
            secp: Secp256k1::new(),
            network,
            resolver: ElectrumClient::new(electrum_url)?,
            wallet_data,
            wallet_file,
        })
    }

    pub fn save(&mut self) -> Result<(), Error> {
        self.wallet_file.seek(io::SeekFrom::Start(0))?;
        self.wallet_data.strict_encode(&self.wallet_file)?;
        Ok(())
    }

    pub fn list_funds(&self) -> Result<Vec<(AddressCompat, u64)>, Error> {
        let map = |(_, (script, utxo_set)): (
            UnhardenedIndex,
            (Script, HashSet<Utxo>),
        )|
         -> Result<(AddressCompat, u64), Error> {
            Ok((
                AddressCompat::from_script(&script, self.network)
                    .ok_or(Error::NoAddressRepresentation)?,
                utxo_set
                    .iter()
                    .map(Utxo::amount)
                    .copied()
                    .map(Amount::as_sat)
                    .sum(),
            ))
        };

        let lookup = |case: UnhardenedIndex,
                      last_index: u32|
         -> Result<Vec<(AddressCompat, u64)>, Error> {
            self.resolver
                .resolve_descriptor_utxo(
                    &self.secp,
                    &self.wallet_data.descriptor,
                    &[case],
                    UnhardenedIndex::zero(),
                    last_index + 20,
                )?
                .into_iter()
                .map(map)
                .collect()
        };

        // Collect normal indexes
        let mut funds = lookup(
            UnhardenedIndex::zero(),
            self.wallet_data.last_normal_index.last_index(),
        )?;
        funds.extend(lookup(
            UnhardenedIndex::one(),
            self.wallet_data.last_change_index.last_index(),
        )?);

        Ok(funds)
    }

    pub fn next_funding_address(&mut self) -> Result<Address, Error> {
        let address = self.wallet_data.descriptor.address(
            &self.secp,
            &[UnhardenedIndex::zero(), self.wallet_data.last_normal_index],
        )?;
        self.wallet_data
            .last_normal_index
            .checked_inc_assign()
            .ok_or(Error::OutOfIndexes)?;
        self.save()?;
        Ok(address)
    }
}

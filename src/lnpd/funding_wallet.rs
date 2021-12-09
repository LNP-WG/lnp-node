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

use std::collections::BTreeMap;
use std::convert::TryInto;
use std::io::Seek;
use std::path::Path;
use std::{fs, io};

use amplify::{IoError, Slice32, ToYamlString};
use bitcoin::secp256k1::{self, Secp256k1};
use bitcoin::{Address, Network, OutPoint, SigHashType, Txid};
use bitcoin_hd::{
    DerivationSubpath, DeriveError, DescriptorDerive, SegmentIndexes, TrackingAccount,
    UnhardenedIndex,
};
use bitcoin_onchain::{ResolveUtxo, UtxoResolverError};
use descriptors::locks::{LockTime, SeqNo};
use descriptors::InputDescriptor;
use electrum_client::{Client as ElectrumClient, ElectrumApi};
use lnp::p2p::legacy::TempChannelId;
use lnpbp::chain::{Chain, ConversionImpossibleError};
use miniscript::Descriptor;
use psbt::construct::Construct;
use psbt::{Psbt, Tx};
use strict_encoding::{StrictDecode, StrictEncode};
use wallet::address::AddressCompat;
use wallet::scripts::PubkeyScript;

/// Errors working with funding wallet
#[derive(Debug, Display, Error, From)]
#[display(doc_comments)]
#[non_exhaustive]
pub enum Error {
    /// error accessing funding wallet file. Details: {0}
    #[from(io::Error)]
    Io(IoError),

    /// error reading or writing funding wallet data. Details: {0}
    #[from]
    StrictEncoding(strict_encoding::Error),

    /// error resolving funding wallet transactions with Electrum server.
    /// Details: {0}
    #[from]
    Electrum(electrum_client::Error),

    /// error resolving funding wallet transactions. Details: {0}
    #[from]
    Resolver(UtxoResolverError),

    /// funding wallet uses custom descriptor which can't be represented as a
    /// valid bitcoin addresses, making channel funding impossible
    NoAddressRepresentation,

    /// chain is not supported for funding
    #[from(ConversionImpossibleError)]
    ChainNotSupported,

    /// chain network mismatches funding wallet network
    ChainMismatch,

    /// unable to derive an address for the descriptor; potentially funding
    /// wallet descriptor is incorrect. Details: {0}
    #[from]
    Derivation(DeriveError),

    /// wallet has run out of funding addresses
    OutOfIndexes,

    /// Insufficient funds for the funding transaction
    InsufficientFunds,

    /// error finalizing transaction, probably not all signatures are present. Details: {0}
    #[from]
    Finalizing(miniscript::psbt::Error),
}

/// Information about funding which is already used in channels pending
/// negotiation or signature
#[derive(Clone, Debug, StrictEncode, StrictDecode)]
#[cfg_attr(
    feature = "serde",
    derive(Display, Serialize, Deserialize),
    serde(crate = "serde_crate"),
    display(PendingFunding::to_yaml_string)
)]
pub struct PendingFunding {
    /// Provsionary channel using some of the funding
    pub temp_channel_id: TempChannelId,
    /// Funding transaction identifier spending one or more of the of available funding UTXOs
    pub funding_txid: Txid,
    /// List of all funding UTXOs which are spent by this prospective channel
    pub prev_outpoints: Vec<OutPoint>,
    /// Partially signed transaction using the funding
    pub psbt: Psbt,
}

#[cfg(feature = "serde")]
impl ToYamlString for PendingFunding {}

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug, StrictEncode, StrictDecode)]
pub struct Funds {
    pub outpoint: OutPoint,
    pub terminal: Vec<UnhardenedIndex>,
    pub script_pubkey: PubkeyScript,
    pub amount: u64,
}

#[derive(Clone, Debug, StrictEncode, StrictDecode)]
#[cfg_attr(
    feature = "serde",
    derive(Display, Serialize, Deserialize),
    serde(crate = "serde_crate"),
    display(WalletData::to_yaml_string)
)]
struct WalletData {
    pub descriptor: Descriptor<TrackingAccount>,
    pub last_normal_index: UnhardenedIndex,
    pub last_change_index: UnhardenedIndex,
    pub last_rgb_index: BTreeMap<Slice32, UnhardenedIndex>,
    pub pending_fundings: BTreeMap<Txid, PendingFunding>,
}

#[cfg(feature = "serde")]
impl ToYamlString for WalletData {}

pub struct FundingWallet {
    secp: Secp256k1<secp256k1::All>,
    network: bitcoin::Network,
    resolver: ElectrumClient,
    wallet_file: fs::File,
    wallet_data: WalletData,
}

impl FundingWallet {
    pub fn new(
        chain: &Chain,
        wallet_path: impl AsRef<Path>,
        descriptor: Descriptor<TrackingAccount>,
        electrum_url: &str,
    ) -> Result<FundingWallet, Error> {
        let wallet_file = fs::File::create(wallet_path)?;
        let wallet_data = WalletData {
            descriptor,
            last_normal_index: UnhardenedIndex::zero(),
            last_change_index: UnhardenedIndex::zero(),
            last_rgb_index: bmap! {},
            pending_fundings: bmap! {},
        };
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
        let wallet_file =
            fs::OpenOptions::new().read(true).write(true).create(false).open(wallet_path)?;
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

    #[inline]
    pub fn network(&self) -> Network { self.network }

    #[inline]
    pub fn descriptor(&self) -> &Descriptor<TrackingAccount> { &self.wallet_data.descriptor }

    /// Scans blockchain for available funds.
    /// Updates last derivation index basing on the scanned information.
    pub fn list_funds(&mut self) -> Result<Vec<Funds>, Error> {
        let used_outputs: Vec<_> = self
            .wallet_data
            .pending_fundings
            .values()
            .flat_map(|funding| &funding.prev_outpoints)
            .collect();

        let lookup =
            |case: UnhardenedIndex, last_index: &mut UnhardenedIndex| -> Result<Vec<_>, Error> {
                Ok(self
                    .resolver
                    .resolve_descriptor_utxo(
                        &self.secp,
                        &self.wallet_data.descriptor,
                        &[case],
                        UnhardenedIndex::zero(),
                        last_index.last_index().saturating_add(20),
                    )?
                    .into_iter()
                    .map(|mut data| {
                        let utxo_set = &mut data.1 .1;
                        *utxo_set = utxo_set
                            .iter()
                            .filter(|utxo| !used_outputs.contains(&utxo.outpoint()))
                            .cloned()
                            .collect();
                        data
                    })
                    .filter(|(_, (_, set))| !set.is_empty())
                    .flat_map(|(index, (script, utxo))| {
                        // Updating last used indexes
                        if index >= *last_index {
                            *last_index = index.checked_inc().unwrap_or_else(UnhardenedIndex::zero);
                        }
                        let script_pubkey = PubkeyScript::from(script.clone());
                        utxo.into_iter().map(move |utxo| Funds {
                            outpoint: *utxo.outpoint(),
                            terminal: vec![case, index],
                            script_pubkey: script_pubkey.clone(),
                            amount: utxo.amount().as_sat(),
                        })
                    })
                    .collect())
            };

        // Collect normal indexes
        let mut last_normal_index = self.wallet_data.last_normal_index;
        let mut last_change_index = self.wallet_data.last_change_index;
        let mut funds = lookup(UnhardenedIndex::zero(), &mut last_normal_index)?;
        funds.extend(lookup(UnhardenedIndex::one(), &mut last_change_index)?);
        self.wallet_data.last_normal_index = last_normal_index;
        self.wallet_data.last_change_index = last_change_index;

        self.save()?;

        Ok(funds)
    }

    pub fn next_funding_address(&self) -> Result<Address, Error> {
        let address = self
            .wallet_data
            .descriptor
            .address(&self.secp, &[UnhardenedIndex::zero(), self.wallet_data.last_normal_index])?;
        Ok(address)
    }

    pub fn construct_funding_psbt(
        &mut self,
        temp_channel_id: TempChannelId,
        address: AddressCompat,
        amount: u64,
        fee: u64,
    ) -> Result<OutPoint, Error> {
        let amount_and_fee = amount + fee;
        // Do coin selection:
        let mut funds = self.list_funds()?;
        funds.sort_by_key(|f| f.amount);
        let sources = funds
            .iter()
            .find(|f| f.amount >= amount_and_fee)
            .map(|elem| vec![elem])
            .or_else(|| {
                let mut acc = 0u64;
                let selection: Vec<_> = funds
                    .iter()
                    .rev()
                    .filter(|last| {
                        acc += last.amount;
                        acc < amount_and_fee
                    })
                    .collect();
                if acc >= amount_and_fee {
                    Some(selection)
                } else {
                    None
                }
            })
            .ok_or(Error::InsufficientFunds)?;

        let inputs = sources
            .into_iter()
            .map(|funds| InputDescriptor {
                outpoint: funds.outpoint,
                terminal: DerivationSubpath::from(funds.terminal.clone()),
                seq_no: SeqNo::with_rbf(0),
                tweak: None,
                sighash_type: SigHashType::All,
            })
            .collect::<Vec<_>>();

        let change_index = self.wallet_data.last_change_index;
        self.wallet_data.last_change_index =
            change_index.checked_inc().unwrap_or_else(UnhardenedIndex::zero);
        let psbt = Psbt::construct(
            &self.secp,
            &self.wallet_data.descriptor,
            LockTime::default(),
            &inputs,
            &[(address.into(), amount)],
            change_index,
            fee,
            &self.resolver,
        )
        .expect("funding PSBT construction is broken");
        let txid = psbt.to_txid();
        self.wallet_data.pending_fundings.insert(txid, PendingFunding {
            temp_channel_id,
            funding_txid: txid,
            prev_outpoints: inputs.iter().map(|inp| inp.outpoint).collect(),
            psbt,
        });

        Ok(OutPoint::new(txid, 0))
    }

    #[inline]
    pub fn get_funding_psbt(&self, txid: Txid) -> Option<&Psbt> {
        self.wallet_data.pending_fundings.get(&txid).map(|funding| &funding.psbt)
    }

    #[inline]
    pub fn publish(&self, mut psbt: Psbt) -> Result<(), Error> {
        miniscript::psbt::finalize(&mut psbt, &self.secp)?;
        let tx = psbt.extract_tx();
        self.resolver.transaction_broadcast(&tx)?;
        Ok(())
    }
}

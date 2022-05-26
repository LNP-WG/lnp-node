// LNP Node: node running lightning network protocol and generalized lightning
// channels.
// Written in 2020-2022 by
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

use amplify::{IoError, Slice32, Wrapper};
use bitcoin::psbt::PartiallySignedTransaction;
use bitcoin::secp256k1::{self, Secp256k1};
use bitcoin::util::bip32::ChildNumber;
use bitcoin::{Address, EcdsaSighashType, Network, OutPoint, Txid};
use electrum_client::{Client as ElectrumClient, ElectrumApi};
use lnp::channel::PsbtLnpFunding;
use lnp::p2p::legacy::TempChannelId;
use lnpbp::chain::{Chain, ConversionImpossibleError};
use miniscript::psbt::PsbtExt;
use miniscript::{Descriptor, DescriptorTrait, ForEachKey};
use strict_encoding::{StrictDecode, StrictEncode};
use wallet::descriptors::locks::{LockTime, SeqNo};
use wallet::descriptors::InputDescriptor;
use wallet::hd::{
    DerivationSubpath, DeriveError, Descriptor as DescriptorExt, SegmentIndexes, TrackingAccount,
    UnhardenedIndex,
};
use wallet::onchain::{ResolveUtxo, UtxoResolverError};
use wallet::psbt::construct::Construct;
use wallet::psbt::Psbt;
use wallet::scripts::PubkeyScript;

// The default fee rate is 2 sats per kilo-vbyte
const DEFAULT_FEERATE_PER_KW: u32 = 2u32 * 1000 * 4;

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

    /// error finalizing transaction, probably not all signatures are present.
    // TODO: Print out details once apmplify library will have `DisplayVec` type.
    Finalizing(Vec<miniscript::psbt::Error>),
}

/// Information about funding which is already used in channels pending
/// negotiation or signature
#[derive(Clone, Debug, StrictEncode, StrictDecode)]
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

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug, StrictEncode, StrictDecode)]
pub struct Funds {
    pub outpoint: OutPoint,
    pub terminal: Vec<UnhardenedIndex>,
    pub script_pubkey: PubkeyScript,
    pub amount: u64,
}

#[derive(Clone, Debug, StrictEncode, StrictDecode)]
struct WalletData {
    pub descriptor: Descriptor<TrackingAccount>,
    pub last_normal_index: UnhardenedIndex,
    pub last_change_index: UnhardenedIndex,
    pub last_rgb_index: BTreeMap<Slice32, UnhardenedIndex>,
    pub pending_fundings: BTreeMap<Txid, PendingFunding>,
}

pub struct FundingWallet {
    secp: Secp256k1<secp256k1::All>,
    network: bitcoin::Network,
    resolver: ElectrumClient,
    feerate_per_kw: u32,
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
        info!("Creating funding wallet at '{}'", wallet_path.as_ref().display());
        let wallet_file = fs::File::create(wallet_path)?;
        let wallet_data = WalletData {
            descriptor,
            last_normal_index: UnhardenedIndex::zero(),
            last_change_index: UnhardenedIndex::zero(),
            last_rgb_index: bmap! {},
            pending_fundings: bmap! {},
        };
        wallet_data.strict_encode(&wallet_file)?;

        let network = chain.try_into()?;
        FundingWallet::init(network, wallet_data, wallet_file, electrum_url)
    }

    pub fn with(
        chain: &Chain,
        wallet_path: impl AsRef<Path>,
        electrum_url: &str,
    ) -> Result<FundingWallet, Error> {
        info!("Opening funding wallet at '{}'", wallet_path.as_ref().display());
        let wallet_file =
            fs::OpenOptions::new().read(true).write(true).create(false).open(wallet_path)?;
        let wallet_data = WalletData::strict_decode(&wallet_file)?;

        let network = chain.try_into()?;
        if DescriptorExt::<bitcoin::PublicKey>::network(&wallet_data.descriptor)? != network {
            return Err(Error::ChainMismatch);
        }

        FundingWallet::init(network, wallet_data, wallet_file, electrum_url)
    }

    fn init(
        network: Network,
        wallet_data: WalletData,
        wallet_file: fs::File,
        electrum_url: &str,
    ) -> Result<FundingWallet, Error> {
        info!("Connecting Electrum server at {}", electrum_url);
        let resolver = ElectrumClient::new(electrum_url)?;

        let mut wallet = FundingWallet {
            secp: Secp256k1::new(),
            network,
            resolver,
            wallet_data,
            wallet_file,
            feerate_per_kw: DEFAULT_FEERATE_PER_KW,
        };
        wallet.update_fees()?;
        Ok(wallet)
    }

    pub fn save(&mut self) -> Result<(), Error> {
        trace!("Saving funding wallet data on disk");
        self.wallet_file.seek(io::SeekFrom::Start(0))?;
        self.wallet_data.strict_encode(&self.wallet_file)?;
        debug!("Funding wallet data is saved on disk");
        Ok(())
    }

    // TODO: Call update fees from a LNPd on a regular basis
    pub fn update_fees(&mut self) -> Result<u32, Error> {
        trace!("Getting fee estimate from the electrum server");
        let fee_estimate = self.resolver.estimate_fee(1)?;
        if fee_estimate == -1.0 {
            debug!(
                "Electrum server was unable to provide fee estimation, keeping current rate of {} \
                 per kilo-weight unit",
                self.feerate_per_kw
            );
        } else {
            self.feerate_per_kw = (fee_estimate * 100_000_000.0 / 4.0) as u32;
            debug!("Updated fee rate is {} per kilo-weight unit", self.feerate_per_kw);
        }
        Ok(self.feerate_per_kw)
    }

    #[inline]
    pub fn network(&self) -> Network { self.network }

    #[inline]
    pub fn descriptor(&self) -> &Descriptor<TrackingAccount> { &self.wallet_data.descriptor }

    #[inline]
    pub fn feerate_per_kw(&self) -> u32 { self.feerate_per_kw }

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
                        let script_pubkey = PubkeyScript::from(script);
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
        let address = DescriptorExt::<bitcoin::PublicKey>::address(
            &self.wallet_data.descriptor,
            &self.secp,
            &[UnhardenedIndex::zero(), self.wallet_data.last_normal_index],
        )?;
        Ok(address)
    }

    pub fn construct_funding_psbt(
        &mut self,
        temp_channel_id: TempChannelId,
        script_pubkey: PubkeyScript,
        amount: u64,
        feerate_per_kw: Option<u32>,
    ) -> Result<Psbt, Error> {
        let feerate_per_kw = feerate_per_kw.unwrap_or(self.feerate_per_kw);
        // We start with the assumption that we will have four-five inputs and two outputs,
        // i.e. it is a 2-kw transaction
        let mut fee_upper_est = 2u64 * feerate_per_kw as u64;
        let amount_and_fee = amount + fee_upper_est;
        // Do coin selection:
        let mut funds = self.list_funds()?;
        funds.sort_by_key(|f| f.amount);

        let mut acc = 0u64;
        let inputs = funds
            .iter()
            .rev()
            .take_while(|funding| {
                if acc >= amount_and_fee {
                    return false;
                }
                acc += funding.amount;
                true
            })
            .map(|funds| InputDescriptor {
                outpoint: funds.outpoint,
                terminal: DerivationSubpath::from(funds.terminal.clone()),
                seq_no: SeqNo::with_rbf(0),
                tweak: None,
                sighash_type: EcdsaSighashType::All,
            })
            .collect::<Vec<_>>();
        if acc < amount_and_fee {
            return Err(Error::InsufficientFunds);
        }

        let change_index = self.wallet_data.last_change_index;
        self.wallet_data.last_change_index =
            change_index.checked_inc().unwrap_or_else(UnhardenedIndex::zero);

        let descriptor = &self.wallet_data.descriptor;

        let mut root_derivations = map![];
        descriptor.for_each_key(|account| {
            let account = account.as_key();
            if let Some(fingerprint) = account.master_fingerprint() {
                root_derivations
                    .insert(account.account_fingerprint(), (fingerprint, &account.account_path));
            }
            true
        });

        let script_pubkey = script_pubkey.into_inner();
        let psbt = loop {
            trace!("Constructing PSBT with fee {}", fee_upper_est);
            let mut psbt: Psbt = Psbt::construct(
                &self.secp,
                descriptor,
                LockTime::default(),
                &inputs,
                &[(script_pubkey.clone().into(), amount)],
                change_index,
                fee_upper_est,
                &self.resolver,
            )
            .expect("funding PSBT construction is broken");
            // Adding full derivation information to each of the inputs
            for input in &mut psbt.inputs {
                for source in input.bip32_derivation.values_mut() {
                    if let Some((fingerprint, path)) = root_derivations.get(&source.0) {
                        source.0 = *fingerprint;
                        source.1 = path
                            .iter()
                            .map(ChildNumber::from)
                            .chain(source.1.into_iter().copied())
                            .collect();
                    }
                }
            }
            psbt.set_channel_funding_output(0).expect("hardcoded funding output number");
            let transaction = psbt.clone().into_transaction();
            // If we use non-standard descriptor we assume its witness will weight 256 bytes per
            // input
            let tx_weight = transaction.weight() as u64;
            let witness_weight = descriptor.max_satisfaction_weight().unwrap_or(256) * inputs.len();
            let precise_fee = (tx_weight + witness_weight as u64) * feerate_per_kw as u64 / 1000;
            if precise_fee == fee_upper_est {
                trace!("Resulting fee matched estimate; exiting PSBT construction cycle");
                break psbt;
            }
            trace!(
                "Resulting fee {} didn't match the target {} reconstructing PSBT",
                precise_fee,
                fee_upper_est,
            );
            fee_upper_est = precise_fee;
        };

        let txid = psbt.to_txid();
        self.wallet_data.pending_fundings.insert(txid, PendingFunding {
            temp_channel_id,
            funding_txid: txid,
            prev_outpoints: inputs.iter().map(|inp| inp.outpoint).collect(),
            psbt: psbt.clone(),
        });

        Ok(psbt)
    }

    #[inline]
    pub fn get_funding_psbt(&self, txid: Txid) -> Option<&Psbt> {
        self.wallet_data.pending_fundings.get(&txid).map(|funding| &funding.psbt)
    }

    #[inline]
    pub fn publish(&self, psbt: Psbt) -> Result<(), Error> {
        let psbt = PartiallySignedTransaction::from(psbt);
        let psbt = psbt.finalize(&self.secp).map_err(|(_, errs)| Error::Finalizing(errs))?;
        let tx = psbt.extract_tx();
        self.resolver.transaction_broadcast(&tx)?;
        Ok(())
    }
}

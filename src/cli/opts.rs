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

use std::net::IpAddr;
#[cfg(feature = "rgb")]
use std::path::PathBuf;
use std::str::FromStr;

use bitcoin::OutPoint;
use internet2::{FramingProtocol, PartialNodeAddr};
use lnp::p2p::legacy::{ChannelId, TempChannelId};
#[cfg(feature = "rgb")]
use rgb::ContractId;

/// Command-line tool for working with LNP node
#[derive(Parser, Clone, PartialEq, Eq, Debug)]
#[clap(name = "lnp-cli", bin_name = "lnp-cli", author, version)]
pub struct Opts {
    /// These params can be read also from the configuration file, not just
    /// command-line args or environment variables
    #[clap(flatten)]
    pub shared: crate::opts::Opts,

    /// Command to execute
    #[clap(subcommand)]
    pub command: Command,
}

impl Opts {
    pub fn process(&mut self) {
        self.shared.process()
    }
}

/// Command-line commands:
#[derive(Subcommand, Clone, PartialEq, Eq, Debug, Display)]
pub enum Command {
    /// Bind to a socket and start listening for incoming LN peer connections
    #[display("listen<{overlay}://{ip_addr}:{port}>")]
    Listen {
        /// IPv4 or IPv6 address to bind to
        #[clap(short, long = "ip", default_value = "0.0.0.0")]
        ip_addr: IpAddr,

        /// Port to use; defaults to the native LN port.
        #[clap(short, long, default_value = "9735")]
        port: u16,

        /// Use overlay protocol (http, websocket etc)
        #[clap(short, long, default_value = "tcp")]
        overlay: FramingProtocol,
    },

    /// Connect to the remote lightning network peer
    Connect {
        /// Address of the remote node, in
        /// '<public_key>@<ipv4>|<ipv6>|<onionv2>|<onionv3>[:<port>]' format
        peer: PartialNodeAddr,
    },

    /// Ping remote peer (must be already connected)
    Ping {
        /// Address of the remote node, in
        /// '<public_key>@<ipv4>|<ipv6>|<onionv2>|<onionv3>[:<port>]' format
        peer: PartialNodeAddr,
    },

    /// General information about the running node
    Info {
        /// Remote peer address or temporary/permanent/short channel id. If
        /// absent, returns information about the node itself
        subject: Option<String>,
    },

    /*
    /// Lists all funds available for channel creation for given list of assets
    /// and provides information about funding points (bitcoin address or UTXO
    /// for RGB assets)
    Funds {
        /// Space-separated list of asset identifiers or tickers. If none are
        /// given lists all avaliable assets
        #[clap()]
        asset: Vec<String>,
    },
     */
    /// Lists existing peer connections
    Peers,

    /// Lists existing channels
    Channels,

    /// Proposes a new channel to the remote peer, which must be already
    /// connected.
    ///
    /// Bitcoins will be added after the channel acceptance with `fund`
    /// command. RGB assets are added to the channel later with `refill``
    /// command
    Propose {
        /// Address of the remote node, in
        /// '<public_key>@<ipv4>|<ipv6>|<onionv2>|<onionv3>[:<port>]' format
        peer: PartialNodeAddr,

        /// Amount of satoshis to allocate to the channel (the actual
        /// allocation will happen later using `fund` command after the
        /// channel acceptance)
        funding_satoshis: u64,
    },

    /// Fund new channel (which must be already accepted by the remote peer)
    /// with bitcoins.
    Fund {
        /// Accepted channel to which the funding must be added
        channel: TempChannelId,

        /// Outpoint (in form of <txid>:<output_no>) which will be used as a
        /// channel funding. Output `scriptPubkey` must be equal to the one
        /// provided by the `propose` command.
        funding_outpoint: OutPoint,
    },

    /// Adds RGB assets to an existing channel
    #[cfg(feature = "rgb")]
    Refill {
        /// Channel to which the funding must be added
        channel: ChannelId,

        /// Consignment file to read containing information about transfer of
        /// RGB20 asset to the funding transaction output
        consignment: PathBuf,

        /// Locally-controlled outpoint (specified when the invoice was
        /// created)
        outpoint: OutPoint,

        /// Outpoint blinding factor (generated when the invoice was created)
        blinding_factor: u64,
    },

    /// Do an invoiceless direct payment
    Transfer {
        /// Channel to which the funding must be added
        channel: ChannelId,

        /// Asset amount to invoice, in atomic unit (satoshis or smallest asset
        /// unit type)
        amount: u64,

        /// Asset ticker in which the invoice should be issued
        #[cfg(feature = "rgb")]
        #[clap(short, long)]
        asset: Option<ContractId>,
    },

    /// Create an invoice
    Invoice {
        /// Asset amount to invoice, in atomic unit (satoshis or smallest asset
        /// unit type)
        amount: u64,

        /// Asset ticker in which the invoice should be issued
        #[clap(default_value = "btc")]
        asset: String,
    },

    /// Pay the invoice
    Pay {
        /// Invoice bech32 string
        #[clap()]
        // TODO: Replace with `Invoice` type once our fix will get merged:
        //       <<https://github.com/rust-bitcoin/rust-lightning-invoice/pull/43>>
        invoice: String,

        /// Channel from which the payment should happen
        #[clap()]
        channel: ChannelId,
    },
}

#[derive(
    Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Display, Error, From,
)]
#[display(doc_comments)]
pub enum AmountOfAssetParseError {
    /// The provided value can't be parsed as a pair of asset name/ticker and
    /// asset amount; use <asset>:<amount> or '<amount> <asset>' form and do
    /// not forget about quotation marks in the second case
    NeedsValuePair,

    /// The provided amount can't be interpreted; please use unsigned integer
    #[from(std::num::ParseIntError)]
    InvalidAmount,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Display)]
#[display("{amount} {asset}", alt = "{asset}:{amount}")]
pub struct AmountOfAsset {
    /// Asset ticker
    asset: String,

    /// Amount of the asset in atomic units
    amount: u64,
}

impl FromStr for AmountOfAsset {
    type Err = AmountOfAssetParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (asset, amount);
        if s.contains(':') {
            let mut split = s.split(':');
            asset = split
                .next()
                .ok_or(AmountOfAssetParseError::NeedsValuePair)?;
            amount = split
                .next()
                .ok_or(AmountOfAssetParseError::NeedsValuePair)?;
            if split.count() > 0 {
                Err(AmountOfAssetParseError::NeedsValuePair)?
            }
        } else if s.contains(' ') {
            let mut split = s.split(' ');
            amount = split
                .next()
                .ok_or(AmountOfAssetParseError::NeedsValuePair)?;
            asset = split
                .next()
                .ok_or(AmountOfAssetParseError::NeedsValuePair)?;
            if split.count() > 0 {
                Err(AmountOfAssetParseError::NeedsValuePair)?
            }
        } else {
            return Err(AmountOfAssetParseError::NeedsValuePair);
        }

        let amount = u64::from_str(amount)?;
        let asset = asset.to_owned();

        Ok(AmountOfAsset { asset, amount })
    }
}

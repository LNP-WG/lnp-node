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

use clap::{AppSettings, Clap};
use std::net::IpAddr;
use std::str::FromStr;

use lnpbp::lnp::{ChannelId, FramingProtocol, PartialNodeAddr};

/// Command-line tool for working with LNP node
#[derive(Clap, Clone, PartialEq, Eq, Debug)]
#[clap(
    name = "lnp-cli",
    bin_name = "lnp-cli",
    author,
    version,
    setting = AppSettings::ColoredHelp
)]
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
#[derive(Clap, Clone, PartialEq, Eq, Debug, Display)]
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
        #[clap()]
        peer: PartialNodeAddr,
    },

    /// Ping remote peer (must be already connected)
    Ping {
        /// Address of the remote node, in
        /// '<public_key>@<ipv4>|<ipv6>|<onionv2>|<onionv3>[:<port>]' format
        #[clap()]
        peer: PartialNodeAddr,
    },

    /// General information about the running node
    Info,

    /// Lists all funds available for channel creation for given list of assets
    /// and provides information about funding points (bitcoin address or UTXO
    /// for RGB assets)
    Funds {
        /// Space-separated list of asset identifiers or tickers. If none are
        /// given lists all avaliable assets
        #[clap()]
        asset: Vec<String>,
    },

    /// Lists existing peer connections
    Peers,

    /// Lists existing channels
    Channels,

    /// Create a new channel with the remote peer, which must be already
    /// connected.
    ///
    /// RGB assets are added to the channel later with FundChannel command
    Create {
        /// Address of the remote node, in
        /// '<public_key>@<ipv4>|<ipv6>|<onionv2>|<onionv3>[:<port>]' format
        #[clap()]
        peer: PartialNodeAddr,

        /// Amount of satoshis to allocate to the channel
        #[clap()]
        satoshis: u64,
    },

    /// Adds RGB assets to an existing channel
    Refill {
        /// Channel to which the funding must be added
        #[clap()]
        channel: ChannelId,

        /// Asset-fund pair
        #[clap()]
        asset: Vec<AmountOfAsset>,
    },

    /// Create an invoice
    Invoice {
        /// Asset amount to invoice, in atomic unit (satoshis or smallest asset
        /// unit type)
        #[clap()]
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

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

use std::net::IpAddr;
use std::str::FromStr;

use internet2::addr::{PartialNodeAddr, ServiceAddr};
use lightning_invoice::Invoice;
use lnp::p2p::bolt::{ChannelId, ChannelType};
use lnp_rpc::LNP_NODE_RPC_ENDPOINT;

/// Command-line tool for working with LNP node
#[derive(Parser, Clone, PartialEq, Eq, Debug)]
#[clap(name = "lnp-cli", bin_name = "lnp-cli", author, version)]
pub struct Opts {
    /// ZMQ socket for connecting daemon RPC interface.
    ///
    /// Socket can be either TCP address in form of `<ipv4 | ipv6>:<port>` – or a path
    /// to an IPC file.
    ///
    /// Defaults to `127.0.0.1:62962`.
    #[clap(
        short = 'R',
        long,
        global = true,
        default_value = LNP_NODE_RPC_ENDPOINT,
        env = "LNP_NODE_RPC_ENDPOINT"
    )]
    pub connect: ServiceAddr,

    /// Set verbosity level.
    ///
    /// Can be used multiple times to increase verbosity.
    #[clap(short, long, global = true, parse(from_occurrences))]
    pub verbose: u8,

    /// Command to execute
    #[clap(subcommand)]
    pub command: Command,
}

/// Command-line commands:
#[derive(Subcommand, Clone, PartialEq, Eq, Debug, Display)]
pub enum Command {
    /// Bind to a socket and start listening for incoming LN peer connections
    #[display("listen<{ip_addr}:{port}>")]
    Listen {
        /// IPv4 or IPv6 address to bind to
        #[clap(short, long = "ip", default_value = "0.0.0.0")]
        ip_addr: IpAddr,

        /// Port to use; defaults to the native LN port.
        #[clap(short, long, default_value = "9735")]
        port: u16,
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

    /// Lists all funds available for channel creation with the list of assets
    /// and provides information about funding points (bitcoin address or UTXO
    /// for RGB assets)
    Funds,

    /// Lists existing peer connections
    Peers,

    /// Lists existing channels
    Channels,

    /// Opens a new channel with a remote peer, which must be already
    /// connected.
    Open {
        /// Address of the remote node, in
        /// '<public_key>@<ipv4>|<ipv6>|<onionv2>|<onionv3>[:<port>]' format
        peer: PartialNodeAddr,

        /// Amount of satoshis to allocate to the channel (the actual
        /// allocation will happen later using `fund` command after the
        /// channel acceptance)
        funding_sat: u64,

        /// Amount of millisatoshis to pay to the remote peer at channel opening
        #[clap(long = "pay")]
        push_msat: Option<u64>,

        // The following are the customization of the channel parameters which should override node
        // settings
        /// Sets fee rate for the channel transacitons.
        ///
        /// If used, overrides default node settings.
        ///
        /// Initial fee rate in satoshi per 1000-weight (i.e. 1/4 the more normally-used 'satoshi
        /// per 1000 vbytes') that this side will pay for commitment and HTLC transactions, as
        /// described in BOLT #3 (this can be adjusted later with an `fee` command).
        #[clap(long)]
        fee_rate: Option<u32>,

        /// Make channel public and route payments.
        ///
        /// If used, overrides default node settings.
        ///
        /// Should the channel be announced to the lightning network. Required for the node to earn
        /// routing fees. Setting this flag results in the channel and node becoming
        /// public.
        #[clap(long)]
        announce_channel: Option<bool>,

        /// Channel type as defined in BOLT-2.
        ///
        /// If used, overrides default node settings.
        ///
        /// Possible values:
        ///
        /// - basic
        ///
        /// - static_remotekey
        ///
        /// - anchored
        ///
        /// - anchored_zero_fee
        #[clap(long)]
        channel_type: Option<ChannelType>,

        /// The threshold below which outputs on transactions broadcast by sender will be omitted.
        ///
        /// If used, overrides default node settings.
        #[clap(long)]
        dust_limit: Option<u64>,

        /// The number of blocks which the counterparty will have to wait to claim on-chain funds
        /// if they broadcast a commitment transaction
        ///
        /// If used, overrides default node settings.
        #[clap(long)]
        to_self_delay: Option<u16>,

        /// The maximum number of the received HTLCs.
        ///
        /// If used, overrides default node settings.
        #[clap(long)]
        htlc_max_count: Option<u16>,

        /// Indicates the smallest value of an HTLC this node will accept, in milli-satoshi.
        ///
        /// If used, overrides default node settings.
        #[clap(long)]
        htlc_min_value: Option<u64>,

        /// The maximum inbound HTLC value in flight towards this node, in milli-satoshi
        ///
        /// If used, overrides default node settings.
        #[clap(long)]
        htlc_max_total_value: Option<u64>,

        /// The minimum value unencumbered by HTLCs for the counterparty to keep in
        /// the channel, in satoshis.
        ///
        /// If used, overrides default node settings.
        #[clap(long)]
        channel_reserve: Option<u64>,
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
        invoice: Invoice,

        /// Channel from which the payment should happen
        channel: ChannelId,

        /// Amount of milli-satoshis to pay. Required for invoices lacking
        /// amount. Overrides amount provided by the invoice.
        amount_msat: Option<u64>,
    },
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Display, Error, From)]
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
            asset = split.next().ok_or(AmountOfAssetParseError::NeedsValuePair)?;
            amount = split.next().ok_or(AmountOfAssetParseError::NeedsValuePair)?;
            if split.count() > 0 {
                return Err(AmountOfAssetParseError::NeedsValuePair);
            }
        } else if s.contains(' ') {
            let mut split = s.split(' ');
            amount = split.next().ok_or(AmountOfAssetParseError::NeedsValuePair)?;
            asset = split.next().ok_or(AmountOfAssetParseError::NeedsValuePair)?;
            if split.count() > 0 {
                return Err(AmountOfAssetParseError::NeedsValuePair);
            }
        } else {
            return Err(AmountOfAssetParseError::NeedsValuePair);
        }

        let amount = u64::from_str(amount)?;
        let asset = asset.to_owned();

        Ok(AmountOfAsset { asset, amount })
    }
}

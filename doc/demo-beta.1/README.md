LNP-node demo
===

### Introduction

This document contains demonstration of `lnp-node` functionality as for [version 0.1.0-beta.1](https://github.com/LNP-BP/lnp-node/releases/tag/v0.1.0-beta.1) and mainly follows [this video-recorded demo]( https://www.youtube.com/watch?v=ns58fUlyd5o).

Two different setups are available:
- [local installation](#local)
- [docker](#docker)

Once either of them is complete, you can proceed with the actual [demo](#demo)

## Local

This setup consists in a local installation of `lnp-node`.

#### Requirements

- [cargo](https://doc.rust-lang.org/book/ch01-01-installation.html#installation)
- [git](https://git-scm.com/downloads)

Furthermore, you will need to install a number of system dependencies:
```bash=
sudo apt install -y build-essential pkg-config libzmq3-dev libssl-dev libpq-dev libsqlite3-dev cmake
```
### Setup

Alongside `lnp-node`, we also need to install `rgb-node` which will be queried for token-related data, then we can launch it in the first terminal (ctrl+C to stop it):
```bash=
git clone https://github.com/LNP-BP/lnp-node
cd lnp-node/doc/demo-beta.1
cargo install --locked --root . rgb_node --version 0.2.0-beta.4 --all-features
cargo install --locked --root . --git "https://github.com/LNP-BP/lnp-node#0.1.0-beta.1" --all-features
./bin/rgbd -vvvv -d ./data -b ./bin/
```
*Note: A single `rgb-node` will serve both `lnp-node`s for simplicity.*

In the second and third terminal you can launch the two `lnp-node` instances:
```bash=
./bin/lnpd -vvvv -d ./data_0 -r lnpz:./data/testnet/fungibled.rpc?api=rpc
./bin/lnpd -vvvv -d ./data_1 -r lnpz:./data/testnet/fungibled.rpc?api=rpc
```
These three terminals will print out logs from the nodes, you can check them out to understand internal workflows. To reduce verbosity, decrease the number of `v` in launch commands.

Now, in the fourth terminal, we can setup aliases to directly access both nodes' command-line interfaces:
```bash=
alias rgb_cli="./bin/rgb-cli -d ./data"
alias lnp0_cli="./bin/lnp-cli -d ./data_0"
alias lnp1_cli="./bin/lnp-cli -d ./data_1"
lnp0_cli help
```
We will need the `node_uri` for both nodes, so we store it in a variable for convenience:
```bash=
node0_uri=$(lnp0_cli info | tr -d '\r' | awk '/node_id/ {print $2"@127.0.0.1"}')
node1_uri=$(lnp1_cli info | tr -d '\r' | awk '/node_id/ {print $2"@127.0.0.1"}')
```
*Note: `| tr -d '\r'` is a workaround for an unexpected `'\r'` character in the output.*

## Docker

In order to create a simple setup that allows to test interactions between `lnp-node`s, we use `docker-compose` together with some helper aliases. You will have CLI access to a couple of `lnp-node`s that will establish connections and channels between them.

#### Requirements

- [git](https://git-scm.com/downloads)
- [docker](https://docs.docker.com/get-docker/)
- [docker-compose](https://docs.docker.com/compose/install/)

### setup
A single `docker-compose up` command will take care of building images and running containers (stop them with `docker-compose down [-v]`):
```bash=
git clone https://github.com/LNP-BP/lnp-node
cd lnp-node/doc/demo-beta.1
# build and run docker containers (takes a while for the first time), use -d to run them in background
docker-compose up [-d]
# to get isolated logs from each node you can for instance run:
docker logs lnp-node-0 # same for rgb-node and lnp-node-1
```
Now we can setup aliases to be able to access nodes' command-line interfaces
```bash=
alias rgb_cli="docker-compose exec rgb-node rgb-cli"
alias lnp0_cli="docker-compose exec lnp-node-0 lnp-cli"
alias lnp1_cli="docker-compose exec lnp-node-1 lnp-cli"
# list of available commands, not all of them are implemented yet
lnp0_cli help
```
We will need the `node_uri` for both nodes, so we store it in a variable for convenience:
```bash=
node0_uri=$(lnp0_cli info | tr -d '\r' | awk '/node_id/ {print $2"@172.1.0.10"}')
node1_uri=$(lnp1_cli info | tr -d '\r' | awk '/node_id/ {print $2"@172.1.0.11"}')
```
*Note: `| tr -d '\r'` is a workaround for an unexpected `'\r'` character in the output.*

## Demo

Once you completed either of the setups above and the three nodes are up and running, you can proceed to the actual demo.

### Premise

Wallet-related functionality is not handled by the nodes, they just perform RGB/LN-specific tasks over data that will be provided by an external wallet such as [bitcoind](https://github.com/bitcoin/bitcoin). In particular, in order to demonstrate a basic workflow, we will need:
- an `issuance_utxo` to which `rgb-node` will bind newly issued asset
- a `change_utxo` on which `rgb-node` receives asset change
- a `funding_outpoint` which `rgb-node` will refill with RGB asset in order to have it available for LN transfers
- a partially signed bitcoin transaction (`transfer_psbt`), whose output pubkey will be tweaked to include a commitment to the transfer.

For the purposes of this demo, since `rgb-node` has no knowledge of the blockchain, we can use "fake" data generated with a testnet or regtest bitcoin node. The following hardcoded UTXOs (that will be used later) will also work:

- `issuance_utxo`: `5aa2d0a8098371ee12b4b59f43ffe6a2de637341258af65936a5baa01da49e9b:0`
- `change_utxo`: `0c05cea88d0fca7d16ed6a26d622e7ea477f2e2ff25b9c023b8f06de08e4941a:1`
- `funding_outpoint`: `79d0191dab03ffbccc27500a740f20a75cb175e77346244a567011d3c86d2b0b:0`
- an example `transfer_psbt` can be found in the `doc/demo-beta.1/samples` folder

### Create a BTC channel

Our first task will be to connect the two nodes as peers and create a lightning channel between them.

First, we connect the nodes as peers:
```bash=
lnp0_cli listen
lnp1_cli connect "$node0_uri"
```
Once the connection is established, either of them can propose a channel to the other party and, after retrieving the temporary channel ID, fund the channel:
```bash=
lnp1_cli propose "$node0_uri" 1000
lnp1_cli info
temp_channel_id=$(lnp1_cli info |grep -A1 '^channels:' |tail -1 |awk '{print $2}')
lnp1_cli info "$temp_channel_id"
# lnp1_cli fund <temp_channel_id> <funding_outpoint>
lnp1_cli fund "$temp_channel_id" 79d0191dab03ffbccc27500a740f20a75cb175e77346244a567011d3c86d2b0b:0
```
*Note: Once the channel gets funded, the `channel_id` changes to its permanent value, though you should still use the temporary ID for the purpose of the demo.*

`lnp-node-1` has now enough balance on the channel to perform the first BTC transfer:
```bash=
lnp1_cli transfer "$temp_channel_id" 20
```
Although the call to lnp-cli create remains hanging (use ctrl-c to exit), the transfer happens correctly as you can see in:
```bash=
lnp1_cli info "$temp_channel_id"
lnp0_cli info "$temp_channel_id"
```

### Add RGB asset to channel

In order to demonstrate RGB functionality over the Lightning Network, we need to allocate some tokens to the `funding_outpoint` and to assign them to the channel via `lnp_cli refill` subcommand. For that purpose an invoice-payment workflow should be performed.

#### Asset issuance

To issue an asset, run:
```bash=
# rgb_cli fungible issue <ticker> <name> <amt>@<issuance_utxo>
rgb_cli fungible issue USDT "USD Tether" 1000@5aa2d0a8098371ee12b4b59f43ffe6a2de637341258af65936a5baa01da49e9b:0
```
This will create a new genesis that includes asset metadata and the allocation of the initial amount to the `<issuance_utxo>`. You can look into it by running:
```bash=
# retrieve <contract-id> with:
rgb_cli genesis list
# export the genesis contract (use -f to select output format)
rgb_cli genesis export <contract-id>
```
You can list known fungible assets with:
```bash=
rgb_cli fungible list
```
which also outputs its `asset-id-bech32`, that is needed to create invoices.

#### Generate invoice

In order to assign some of the new USDT to the channel, `rgb-node` needs to generate an invoice towards the funding outpoint:
```bash=
# rgb_cli fungible invoice <asset-id-bech32> 100 <funding_outpoint>
rgb_cli fungible invoice <asset-id-bech32> 100 79d0191dab03ffbccc27500a740f20a75cb175e77346244a567011d3c86d2b0b:0
```
This outputs `invoice` and `blinding_factor`.

To be able to refill the channel with USDT tokens, we will need the `funding_outpoint` and the corresponding `blinding_factor` that was used to include it in the invoice.

#### Transfer asset to the funding outpoint

To transfer asset to the `funding_outpoint`, `rgb-node` needs to create a consignment and commit to it into a bitcoin transaction. So we will need the invoice and a partially signed bitcoin transaction that will be modified to include the commitment. Furthermore, `-i` and `-a` options allow to provide an input UTXO from which to take asset and an allocation for the change in the form `<amount>@<utxo>`.

```bash=
# NB: pass the invoice between quotes to avoid misinterpretation of the & character into it
# rgb_cli fungible transfer '<invoice>' </path/to/source.psbt> </where/to/store/consignment.rgb> </where/to/store/witness.psbt> -i <issuance_utxo> -a 900@<change_utxo>
rgb_cli fungible transfer '<invoice>' samples/source_tx.psbt samples/consignment.rgb samples/witness.psbt \
-i 5aa2d0a8098371ee12b4b59f43ffe6a2de637341258af65936a5baa01da49e9b:0 \
-a 900@0c05cea88d0fca7d16ed6a26d622e7ea477f2e2ff25b9c023b8f06de08e4941a:1
```
This will write the consignment file and the PSBT including the tweak (which is called *witness transaction*) at the provided paths.

At this point, in a real setting, the witness transaction should be signed and broadcast, while the consignment is sent off-chain to the peer.

#### Refill channel

*Note: `rgb-node` methods `validate` and `accept` are performed in the background when `refill` is called.*

Once the asset transfer is completed, we need the two parties of the channel to acknowledge this new asset and commit to it at the LN level. This is done via `lnp_cli refill` subcommand; this requires the `temp_channel_id`, the consigment file we stored into the `samples` directory, the `funding_outpoint` and the `blinding_factor` which was obtained at invoice creation.
```bash=
# lnp1_cli refill <temp_channel_id> </path/to/consignment.rgb> <funding_outpoint> <blinding_factor>
lnp1_cli refill "$temp_channel_id" samples/consignment.rgb 79d0191dab03ffbccc27500a740f20a75cb175e77346244a567011d3c86d2b0b:0 <blinding_factor>
```

#### Transfer asset

We are finally ready for our first RGB asset transfer over the lightning network:
```bash=
# get asset-id-hex from here
rgb_cli genesis list
lnp1_cli transfer --asset <asset-id-hex> "$temp_channel_id" 10
```
Although the call to lnp-cli create remains hanging (use ctrl-c to exit), the transfer happens correctly as you can see in:
```bash=
lnp1_cli info "$temp_channel_id"
lnp0_cli info "$temp_channel_id"
```

*Note: different encodings for the `asset-id` are used for historical reasons, but will be uniformed to `bech-32` in the future; progress is tracked in [this dedicated issue](https://github.com/LNP-BP/lnp-node/issues/33).*

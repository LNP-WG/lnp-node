# LNP node alpha.4 demo

### Introduction
This document contains a textual version of the [LNP Node Alpha 4 demo video]( https://www.youtube.com/watch?v=TgmyO0ecVNI&feature=youtu.be), based on the [0.1 Aplha 4 release](https://github.com/LNP-BP/lnp-node/releases/tag/v0.1.0-alpha.4). It is meant to demonstrate initial functionality of the node and its interface (for additional information on the functionality check out the release notes).

For troubleshooting check out this [issue](https://github.com/LNP-BP/lnp-node/issues/22#event-3959465543)

Two different setups are available:
- [local installation](#local)
- [docker](#docker)

Once either of them is complete, you can proceed with the actual [demo](#demo)

## Local
This setup describes a local installation of the `lnp-node`.

#### Requirements
- [cargo](https://doc.rust-lang.org/book/ch01-01-installation.html#installation)

### Setup
**Note:** *there is no need to use git or download the code, it's already in Rust crate repository and everything is being done via cargo command*

In the first terminal, to install `lnp-node` and launch the first instance, run: 
```bash=
cargo install lnp_node --vers 0.1.0-alpha.4 --all-features
lnpd -vvvv -d ./data_dir_0
```

In the second terminal you can launch the second instance:
```bash=
lnpd -vvvv -d ./data_dir_1
```
These two terminals will print out logs from the two nodes, you can check them out to understand internal workflows. To reduce verbosity, decrease the number of `v` in launch commands.

Now, in the third terminal, we can setup aliases to directly access both nodes' command-line interfaces:
```bash=
alias lnp0-cli="lnp-cli -d ./data_dir_0"
alias lnp1-cli="lnp-cli -d ./data_dir_1"
# list of available commands, not all of them have been implemented yet
lnp0-cli help
```
We will need the node_uri for both nodes, so we store it in a variable for convenience:
```bash=
node0_uri=$(lnp0-cli info | awk '/node_id/ {print $2"@127.0.0.1"}')
node1_uri=$(lnp1-cli info | awk '/node_id/ {print $2"@127.0.0.1"}')
```


## Docker

In order to create a simple setup that allows you to test the interactions between lnp-nodes, we use `docker-compose` together with some helper aliases. You will have CLI access to a couple of lnp-nodes that will establish connections and channels between them.

#### Requirements
- [git](https://git-scm.com/downloads)
- [docker](https://docs.docker.com/get-docker/)
- [docker-compose](https://docs.docker.com/compose/install/)

### Setup
```bash=
git clone https://github.com/LNP-BP/lnp-node
cd lnp-node/doc/dempo-alpha.4
# build lnp-node docker image (it takes a while...)
docker build -t lnp-node:v0.1.0-alpha.4 .
# run docker containers, use -d to run them in background
docker-compose up [-d]
# to get isolated logs from each node you can for instance run:
docker logs lnp-node-0
```
Now we can setup aliases to be able to access both nodes' command-line interfaces:
```bash=
alias lnp0-cli="docker exec lnp-node-0 lnp-cli"
alias lnp1-cli="docker exec lnp-node-1 lnp-cli"
# list of available commands, not all of them are implemented yet
lnp0-cli help
```
We will need the node_uri for both nodes, so we store it in a variable for convenience:
```bash=
node0_uri=$(lnp0-cli info | awk '/node_id/ {print $2"@172.1.0.10"}')
node1_uri=$(lnp1-cli info | awk '/node_id/ {print $2"@172.1.0.11"}')
```

## Demo

Once you have completed either of the setups above and the two nodes are up and running, you can proceed to the actual demo.

### Create a channel
Our task will be to connect the two nodes as peers and create a Lightning channel between them. 

First, we connect the nodes as peers:
```bash=
lnp0-cli listen
lnp1-cli connect "$node0_uri"
```
Once the connection is established, either of them can initialize channel creation
```bash=
lnp1-cli create "$node0_uri" 123
lnp0-cli create "$node1_uri" 1234
```
**Note:** *although the call to `lnp-cli create` remains hanging ([we are trying to solve it](https://github.com/LNP-BP/lnp-node/issues/25)), the channel is created: the `channels` field in `lnp0-cli info` increases correctly.*

### list of available commands


```
$ lnp0-cli help
FLAGS:
    -h, --help       Prints help information
    -v, --verbose    Set verbosity level
    -V, --version    Prints version information

OPTIONS:
    -n, --chain <chain>              Blockchain to use [env: LNP_NODE_NETWORK=] [default: signet]
    -c, --config <config>            Path to the configuration file [env: LNP_NODE_CONFIG=]
    -x, --ctl-socket <ctl-socket>    ZMQ socket name/address for daemon control interface [env:
                                     LNP_NODE_CTL_SOCKET=] [default:
                                     lnpz:{data_dir}/ctl.rpc?api=esb]
    -d, --data-dir <data-dir>        Data directory path [env: LNP_NODE_DATA_DIR=] [default:
                                     ~/.lnp_node]
    -m, --msg-socket <msg-socket>    ZMQ socket name/address to forward all incoming lightning
                                     messages [env: LNP_NODE_MSG_SOCKET=] [default:
                                     lnpz:{data_dir}/msg.rpc?api=esb]
    -T, --tor-proxy <tor-proxy>...      Use Tor [env: LNP_NODE_TOR_PROXY=]

SUBCOMMANDS:
    channels    Lists existing channels
    connect     Connects to the remote lightning network peer
    create      Creates a new channel with the remote peer, which must be already connected
    funds       Lists all funds available for channel creation for given list of assets and
                provides information about funding points (bitcoin address or UTXO for RGB
                assets)
    help        Prints the current message or the help-message of the given subcommand(s)
    info        General information about the running node
    invoice     Creates an invoice
    listen      Binds to a socket and starts listening to incoming LN peer connections
    pay         Pays the invoice
    peers       Lists existing peer connections
    ping        Pings remote peer (must be already connected)
    refill      Adds RGB assets to an existing channel
```

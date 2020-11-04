
In first terminal (**NB: no need to use git or download the code, it's already in Rust crate repository and all is done by `cargo` command**):
```shell script
cargo install lnp_node --vers 0.1.0-alpha.4 --all-features
lnpd -vvv
```

In second terminal:
```shell script
lnpd -vvv -d /tmp
```

In third terminal:
```console
$ lnp-cli help
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
    connect     Connect to the remote lightning network peer
    create      Create a new channel with the remote peer, which must be already connected
    funds       Lists all funds available for channel creation for given list of assets and
                provides information about funding points (bitcoin address or UTXO for RGB
                assets)
    help        Prints this message or the help of the given subcommand(s)
    info        General information about the running node
    invoice     Create an invoice
    listen      Bind to a socket and start listening for incoming LN peer connections
    pay         Pay the invoice
    peers       Lists existing peer connections
    ping        Ping remote peer (must be already connected)
    refill      Adds RGB assets to an existing channel

$ lnp-cli info
$ lnp-cli -d /tmp info
$ lnp-cli -d /tmp listen
$ lnp-cli connect <node_id_from_info>@127.0.0.1
$ lnp-cli info
$ lnp-cli connect <node_id_from_info>@127.0.0.1 100
```

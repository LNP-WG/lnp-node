# LNP Node: Lightning Network Protocol Node

LNP Node is a new Lightning Network node written from scratch in Rust. 
Actually, it's a suite of daemons/microservices able to run both Lightning 
Network (LN) as it is defined in BOLT standards - and generalized lightning 
codenamed "Bifrost": a full refactoring of the lightning network protocols
supporting Taproot, Schnorr signatures, RGB assets, DLCs, multi-peer channels,
channel factories/channel composability and many other advanced features.
LNP Node operates using Internet2 networking protocols and specially-designed
microservice architecture.

One may ask: another LN node? Why we need it? And what is this lightning 
network protocol (LNP), and Internet2 networking and generalization of lightning
channels coming with Bifrost?

**Lightning Network Protocol** is unification of both "normal" (or "legacy")
Lightning network and Bifrost: a family of protocols operating on top of
bitcoin protocols (BP) and together composing family of LNP/BP tech stack.

**Internet2** is a set of best practices to use existing protocols in network 
communications preserving most of privacy and anonymity. It supports both P2P
and RCP (client-server) operations and represents lightning-styled Noise_XK
encrypted communications (instead of SSL/TLS) for P2P and encrypted ZeroMQ
for client-server communications. This allows to avoid most common pitfalls
with centralized certificate authorities or plain-text low-efficiency JSON/XML
RPC or web service protocols. P2P layer is an extract from BOLT-8 and, 
partially, BOLT-1, extended to support arbitrary messaging, RPC, P2P and publish/
subscribe APIs over TCP/IP, TCP/Tor, UDP, ZeroMQ and high latency communication
channels (mesh/satellite networks) with end-to-end encryption. It is 100% LN
compatible (and in these terms Lightning Network runs on this protocol de-facto)
but allows much more than current LN uses. The protocol is defined as a set of
LNPBP15-19 standards, which are strict extensions of BOLTs. In other words, with 
this protocol you can do an arbitrary messaging (and build complex distributed
systems without central authorities like DNS, SSL, IP addresses), so at LNP/BP
Association we use it everywhere, even for internal communications between 
microservices.

**Bifrost** is a way of defining payment channels in a modular and extensible 
way such that you can easily add new transaction outputs to the commitment 
transaction, switch from HTLCs to PTLCs payments, use taproot & do a lot of
experimentation without inventing new messages and standards each time: peers are
using Bifrost to negotiate channel and transaction structure with 
partially-signed transactions.

Idea for both protocols came from Dr Maxim Orlovsky, Dr Christian Decker and
Giacomo Zucco discussions in 2019-2020 and implemented by Maxim Orlovsky as a 
part of [LNP Core Library](https://github.com/LNP-BP/lnp-core).
We recommend to watch to [Potzblitz about LNP Node](https://www.youtube.com/watch?v=YmmNsWS5wiM&t=5s&ab_channel=Fulmo%E2%9A%A1)
and [LNP/BP networking presentation](https://www.youtube.com/watch?v=kTwZKsbIPbc&t=2123s&ab_channel=LNPBPStandardsAssociation)
to get a deeper insight into these topics. Presentations slides are also 
avaliable:
* [LNP/BP Decentralization Solutions]()
* [Future of the Lightning Network]() (slides from the Postblitz talk)

## Rationale & features

The problem with the existing Lightning node implementations is their very 
limited extensibility for such things as:

* Future LN upgrades ([channel factories](https://tik-old.ee.ethz.ch/file//a20a865ce40d40c8f942cf206a7cba96/Scalable_Funding_Of_Blockchain_Micropayment_Networks%20(1).pdf),
  [payment points](https://suredbits.com/payment-points-part-1/), Taproot),
  since they do not separate network communication, channel operation and 
  channel parameters from each other in a well manner, such that it will be 
  possible, for instance, to replace HTLCs with payment points using some 
  extension/module;
* Protocols on top of LN (layer 3), like [RGB], DLCs or proposed
  [Lightspeed protocol](https://github.com/LNP-BP/lnpbps/issues/24), which 
  require modification on the structure of the commitment transaction;
* Custom non-payment channel types, for instance trustless storage with [Storm]   
  or computing with [Prometheus].

We name the extensions to Lightning network required to build this rich 
functionality "Bifrost". With this project 
[LNP/BP Standards Association](https://github.com/LNP-BP) is trying to build an 
LN node with extensible and highly-modular architecture, utilizing 
state-of-the-art Rust approaches like:
* Mobile-, cloud & web-ready, due to a specially-designed 
  [microservice architecture](#approach)
* Dockerization for scalability at the level of separate processes (per-channel 
  scalability etc)
* Fast and performant ZeroMQ for APIs and IPCs.

This new node will be used to implement:

* Bidirectional channels
* [Channel factories/multipeer channels](https://tik-old.ee.ethz.ch/file//a20a865ce40d40c8f942cf206a7cba96/Scalable_Funding_Of_Blockchain_Micropayment_Networks%20(1).pdf);
* [Payment points](https://suredbits.com/payment-points-part-1/);
* [DLCs on LN](https://hackmd.io/@lpQxZaCeTG6OJZI3awxQPQ/LN-DLC);
* [RGB] smart contracts (client-validated smart contract system);
* Future [Storm] – storage & messaging state channels;
* Future [Prometheus] – high-load computing state channels;
* [Lightspeed payment protocol](https://github.com/LNP-BP/lnpbps/issues/24);
* Schnorr's/Taproot.

## Design

### Approach

The node (as other nodes maitained by LNP/BP Standards Association and Pandora
Core company subsidiaries) consists of multiple microservices, communicating
with each other via ZMQ RPC interfaces.

![Node architacture](doc/node_arch.jpeg)

The set of microservices representing node can run as either:
1) single daemon process on desktop or a server;
2) cloud of docker-based daemons, one per microservice, with instance 
   scalability and in geo-distributed environment;
3) inside a single mobile app as threads;
4) and even different nodes can be combined in their services between themselves
   into a single executables/mobile apps;
5) all P2P communications are end-to-end encrypted and work over Tor.

Other nodes, designed an maintained by LNP/BP Standards Association with the 
same architecture include:
* [RGB Node](https://github.com/LNP-BP/rgb-node) for running RGB smart contracts
  over bitcoin and lightning network
* [BP Node](https://github.com/LNP-BP/bp-node) for indexing bitcoin blockchain
  (you may think of it as a more efficient Electrum server alternative)

Other third parties provide their own nodes:
* [MyCitadel](https://github.com/mycitadel/mycitadel-node) Bitcoin, LN & RGB
  enabled wallet service with support for other LNP/BP protocols;
* [Keyring](https://github.com/pandoracore/keyring) for managing private key
  accounts, storage and signatures with support for miniscript and PSBTs.

### LNP Node Architecture Specifics

The overall architecture of LNP Node is the following:

![Node architacture](doc/lnp_node_arch.jpeg)

More information on the service buses used in the node:

![Node architacture](doc/node_esb.jpeg)


## Project organization & architecture

* [`cli/`](cli/src) – command line API talking to LNP Node via RPC (see below);
* [`rpc/`](rpc/src) – RPC client library for controlling LNP Node;
* [`src/`](src) – main node source code:
  - [`peerd/`](node/src/peerd) – daemon managing peer connections 
    within Lightning peer network using LNP (Lightning network protocol).;
  - [`channeld`](node/src/channeld) – daemon managing generalized Lightning
    channels with their extensions;
  - [`lnpd`](node/src/lnpd) – daemon initializing creation of new channels and
    connections;
  - [`routed`](node/src/routed) – daemon managing routing & gossips;
  - [`watchd`](node/src/watchd) – daemon watching on-chain transaction status;
  - [`signd`](node/src/signd) - key managing for key derivation & signatures;
    uses [Descriptor Wallet lib](https://github.com/LNP-BP/descriptor-wallet).

Each daemon (more correctly "microservice", as it can run as a thread, not 
necessary a process) or other binary (like CLI tool) follows the same
organization concept for module/file names:
* `error.rs` – daemon-specific error types;
* `opts.rs` – CLAP arguments & daemon configuration data;
* `runtime.rs` – singleton managing main daemon thread and keeping all ZMQ/P2P 
  connections and sockets; receiving and processing messages through them;
* `automata/` - state machines implementing different operation workflows;
* `index/`, `storage/`, `cache/` – storage interfaces and engines;
* `db/` – SQL-specific schema and code, if needed.

## Build and usage

### Dependencies

To compile the node, please install [cargo](https://doc.rust-lang.org/cargo/)

```bash
sudo apt install -y build-essential cmake libsqlite3-dev libssl-dev libzmq3-dev pkg-config
cargo install --path . --locked --all-features
```

### Generate funding wallet (First Time)

Before running the node, it is necessary to set an _xpriv_ to create a funding wallet:

```bash
lnpd -vvv init

# The prompt shows something like this:
Initializing node data
Data directory '/[DATA_DIR]/.lnp_node/signet' ... found
Signing account 'master.key' ... creating
Please enter your master xpriv:

```

### Running Local

To compile the node, please install [cargo](https://doc.rust-lang.org/cargo/),
then run the following commands:

```bash
sudo apt install -y build-essential cmake libsqlite3-dev libssl-dev libzmq3-dev pkg-config
cargo install --path . --locked --all-features
lnpd -vvv
```

### Running in docker

```bash
docker build -t lnp-node .
docker run --rm --name lnp_node lnp-node
```

## Ways of communication

* IRC channels on Freenode
    * \#lnp-bp: discussions on this and other LNP/BP projects
    * \#rust-bitcoin: here you can get general support on rust-lightning
    * \#lightning-dev: here better to ask generic lightning-network questions
    * dr_orlovsky: my account on IRC
* Lightning Hackdays Mattermost channel:
  <https://mm.fulmo.org/fulmo/channels/x-rust-ln-node>

[Storm]: https://github.com/storm-org/storm-spec
[Prometheus]: https://github.com/pandoracore/prometheus-spec/blob/master/prometheus.pdf
[RGB]: https://github.com/rgb-org/

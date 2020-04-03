# lnpd: Generalized Lightning Network node

[![TravisCI](https://api.travis-ci.com/LNP-BP/lnpd.svg?branch=master)](https://api.travis-ci.com/LNP-BP/lnpd)

`lnpd` is a new Lightning Network node written in Rust. Actually, it's a suite of daemons able to run generalized Lightning Network protocol.

One may ask: another LN node? Why we need it? And what is "generalized Lightning Network"?

The problem with the existing Lightning node implementations is their very limited extensibility for such things as:

* future LN upgrades ([channel factories](https://tik-old.ee.ethz.ch/file//a20a865ce40d40c8f942cf206a7cba96/Scalable_Funding_Of_Blockchain_Micropayment_Networks%20(1).pdf), [payment points](https://suredbits.com/payment-points-part-1/), Taproot), since they do not separate network communication, channel operation and channel parameters from each other in a well manner, such that it will be possible, for instance, to replace HTLCs with payment points using some extension/module.
* protocols on top of LN (layer 3), like DLCs or proposed [Lightspeed protocol](https://github.com/LNP-BP/lnpbps/issues/24), which require modification on the structure of the commitment transaction.

We name the extensions to Lightning network required to build this rich functionality a "Generalized Lightning Network". With this project [LNP/BP Standards Association](https://github.com/LNP-BP) is trying to build an LN node with extensible and highly-modular architecture, utilizing state of the art Rust approaches like:

* Microservice architecture
* Dockerization for scalability at the level of separate processes (per-channel scalability etc)
* Tokio-based async/non-blocking IO and rumtime
* Fast and performant ZeroMQ for APIs and IPCs
* Avoiding non-efficient Bitcoin blockchain parsing and instead relying on new [scalable blockchain indexing service](https://github.com/LNP-BP/txserv) and new format of [universal bitcoin identifiers](https://github.com/LNP-BP/lnpbps/blob/master/lnpbp-0005.md)
* Mobile- & web-ready via C- and WASM-bindings & build targets for the core components

This new node will be used to implement:

* Bidirectional channels
* [Channel factories/multipeer channels](https://tik-old.ee.ethz.ch/file//a20a865ce40d40c8f942cf206a7cba96/Scalable_Funding_Of_Blockchain_Micropayment_Networks%20(1).pdf)
* [Payment points](https://suredbits.com/payment-points-part-1/)
* [DLCs on LN](https://hackmd.io/@lpQxZaCeTG6OJZI3awxQPQ/LN-DLC)
* [RGB & Spectrum](https://github.com/rgb-org/spec)
* Future [Storm](https://github.com/storm-org/storm-spec) (storage & messaging) edition for LN
* Future [Prometheus](https://github.com/pandoracore/prometheus-spec/blob/master/prometheus.pdf) (high-load computing) edition for LN
* [Lightspeed payment protocol](https://github.com/LNP-BP/lnpbps/issues/24)

The node must maintain simple/modular upgradability for:

* Schnorr's/Taproot
* Pay-to-elliptic curve point replacement for HTLCs
* eltoo


## Previous work & docs

* `rust-lightning` architecture explanation: <https://github.com/rust-bitcoin/rust-lightning/blob/master/ARCH.md>
* Antoine Riard's explanation on possible LN daemon architecture: <https://github.com/ariard/hacking-rust-lightning>
* PR in rust-lightning removing storage of private keys from the memory: <https://github.com/rust-bitcoin/rust-lightning/pull/562/files>
* Matt Corallo's sample implementation of rust-lightning based daemon, showcasing its API: <https://github.com/TheBlueMatt/rust-lightning-bitcoinrpc>
* Antoine Riard's set of separate lightning daemons based on standard Rust multithreading/async (no tokio): <https://github.com/ariard/orage>
* Forks of Matt Corallo's daemon developed much further towards full-flegged Lightning daemoin:
    * by @YaaMe: <https://github.com/YaaMe/rust-lightning-bitcoinrpc>
    * further development by @knarfytrebil: <https://github.com/knarfytrebil/rust-lightning-bitcoinrpc>
* Sample of Rust multithreaded server architecture: <https://github.com/LNP-BP/txserv>


## Ways of communication

* Lightning Hackdays Mattermost channel: <https://mm.fulmo.org/fulmo/channels/x-rust-ln-node>
* IRC channels on Freenode
    * \#rust-bitcoin: here you can get general support on rust-lightning
    * \#lightning-dev: here better to ask generic lightning-network questions
    * \#lnp-bp: discussions on this and other LNP/BP projects
    * dr_orlovsky: my account on IRC
* Slack channel of SquareCrypto devoted to the development of Lightning Development Kit based on `rust-lightning` library: <http://lightningdevkit.org/>

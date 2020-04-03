# lnpd: Generalized Lightning Network node

`lnpd` is a new Lightning Network node written in Rust. Actually, it's a suite of daemons able to runn generalized Lightning Network protocol.

One may ask: another LN node? Why we need it? And what is "generalized Lightning Network"?

The problem with the existing Lightning node implementations is their very limited extensibility for such things as:

* future LN upgrades (channel factories,  pay-to-ec-point, taproot), since they do not separating network communication, channel operation and channel parameters well (such that it will be possible to replace HTLCs with payment points using some extension/module)
* protocols on top of LN (layer 3), like DLCs or proposed [Lightspeed protocol](https://github.com/LNP-BP/lnpbps/issues/24), which require modification on the structure of the commitment transaction.

We name the extensions to Lightning network required to build this rich functionality a "Generalized Lightning Network". With this project [LNP/BP Standards Association](https://github.com/LNP-BP) is trying to build an LN node with extensible and highly-modular architecture, utilizing state of the art Rust approaches like:

* ZeroMQ for APIs and IPCs
* Microservice architecture
* Dockerization for scalability at the level of separate processes (per-channel scalability etc)
* Tokio-based async/non-blocking IO and rumtime
* Avoiding non-efficient Bitcoin blockchain parsing and instead relying on new [scalable blockchain indexing service](https://github.com/LNP-BP/txserv) and new format of [universal bitcoin identifiers](https://github.com/LNP-BP/lnpbps/blob/master/lnpbp-0005.md)
* Mobile- & web-ready via C- and WASM-bindings & build targets for the core components

This new node will be used to implement:

* Bidirectional channels
* Channel factories/multipeer channels
* DLCs on LN
* RGB & Spectrum
* Storm (storage & messaging) on LN
* Prometheus on LN
* [Lightspeed payment protocol](https://github.com/LNP-BP/lnpbps/issues/24)

The node must maintain simple/modular upgradability for:

* Schnorr's/Taproot
* Pay-to-elliptic curve point replacement for HTLCs
* eltoo

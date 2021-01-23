Change Log
==========

v0.1.0-beta.3
-------------
- Support for lightning peer message encoding (before used custom strict-encding)
- Better representation of log messages
- Migration on v0.3 of LNP/BP Core libraries after their complete refactoring

v0.1.0-beta.2
-------------
- Fix balances consistency in channeld
- README & build instructions improvements
- Modernizing for LNP/BP Core v0.2 release & RGB Node v0.2 RC

v0.1.0-beta.1
--------------
- Channel funding
- Channel operations, HTLCs
- RGB node integration
- Adding assets to the channel (funding with assets)
- Transfers of RGB assets

v0.1.0-alpha.4
--------------
- Daemon management with LNPd
- Connection initialisation cycle
- Much better reporting in console tool
- Completed set of command-line management & information commands
- Improved channel creation lifecycle
- Multiple improvements to debugging information representation

v0.1.0-alpha.3
--------------
- Channel negotiation between nodes
- Reworked service buses; added inter-daemon routing
- Separated general service runtime functionality

v0.1.0-alpha.2
--------------
- Skeleton for lnpd, channeld, gossipd and routed services/daemons
- Ping/pong interplay betweeen nodes
- Completed implementatino of enterprise service buses (CTL, MSG, BRIDGE)
- Basic channel creating workflow

v0.1.0-alpha.1
--------------
Initial pre-release:
- Multithreaded concurrent architecture
- Connection daemon
- Command-line tool
- LN peer connectivity
- ZMQ RPC command buses: for LN messages and control


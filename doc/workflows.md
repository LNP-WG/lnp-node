# Command flows

## Connect peer
1. Local flow
  - user->cli: `connect <peer>` command
  - cli->lnpd: `ConnectPeer`
  - lnpd: launches `connectiond` with remote peer specified
  - connectiond->lnpd: `Hello`
	- lnpd: registers channeld
  - connectiond: establishes TCP connection with the remote peer
  - connectiond: sends remote peer `Init` message
2. Remote flow
  - connectiond-listener: accepts TCP connection and spawns new connectiond instance
  - connectiond->lnpd: `Hello`
	- lnpd: registers channeld
	- connectiond: receives `Init` message
	- #TODO connectiond->lnpd: forwards `Init` message
	- #TODO lnpd: verifies assets from the init message asking fungibled on provided assets via extension mechanism
3. #TODO If some of the assets or features not supported
  - lnpd->connectiond: `Terminate`
  - lnpd: removes connectiond from the list of available connections
  - connectiond: terminates connection and shuts down
  - lnpd: force killing in 1 sec

## Ping-pong
1. Local flow
  - tcp->connectiond: Timeout
  - connectiond: checks that the preivous `pong` was responded, otherwise marks the remote peer unresponding
  - connectiond: sends remote peer `Ping` message
2. Remote flow
  - connectiond: receives `Ping` message
  - connectiond: prepares and sends `Pong` response
3. Local flow
  - connectiond: receives `Pong` message and proceeds

## Channel creation
1. Local flow
	- user->cli: `create channel <peer>` command
	- cli->lnpd: `OpenChannelWith`
	- lnpd: launches `channeld` and waits for it's connection
	- channeld->lnpd: `Hello`
	- lnpd: registers channeld
	- lnpd->channeld: `OpenChannelWith`
	- channeld->connectiond: `OpenChannel` message
	- connectiond: sends remote peer `OpenChannel` message
2. Remote flow
	- connectiond: receives `OpenChannel` message
	- connectiond->lnpd: forwards `OpenChannel` message
	- lnpd: launches `channeld` and waits for it's connection
	- channeld->lnpd: `Hello`
	- lnpd: registers channeld
	- lnpd->channeld: `AcceptChannelFrom`
	- channeld->connectiond: `AcceptChannel` message
	- connectiond: sends remote peer `AcceptChannel` message
3. Local flow
	- connectiond: receives `AcceptChannel` message
	- connectiond->channeld: forwards `AcceptChannel` message
	- channeld: marks channel as accepted
4. #TODO Continue:
	* local->remote: FundingCreated
	* remote->local: FundingSigned
	* local<->remote: FundingLocked

## #TODO Payment
1. Local flow
  - user->cli: `pay <invoice> <channel>` command-line command
  - cli->channeld: sends `PayInvoice`
  - channeld: checks balances etc
	- channeld->connectiond: `UpdateAddHtlc` message
  - 
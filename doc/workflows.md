# Command flows

## Connect peer
1. Local flow
  - user->cli: `connect <peer>` command
  - cli->lnpd: `ConnectPeer`
  - lnpd: launches `peerd` with remote peer specified
  - peerd->lnpd: `Hello`
	- lnpd: registers channeld
  - peerd: establishes TCP connection with the remote peer
  - peerd: sends remote peer `Init` message
2. Remote flow
  - peerd-listener: accepts TCP connection and spawns new peerd instance
  - peerd->lnpd: `Hello`
	- lnpd: registers channeld
	- peerd: receives `Init` message
	- #TODO peerd->lnpd: forwards `Init` message
	- #TODO lnpd: verifies assets from the init message asking fungibled on provided assets via extension mechanism
3. #TODO If some of the assets or features not supported
  - lnpd->peerd: `Terminate`
  - lnpd: removes peerd from the list of available connections
  - peerd: terminates connection and shuts down
  - lnpd: force killing in 1 sec

## Ping-pong
1. Local flow
  - tcp->peerd: Timeout
  - peerd: checks that the preivous `pong` was responded, otherwise marks the remote peer unresponding
  - peerd: sends remote peer `Ping` message
2. Remote flow
  - peerd: receives `Ping` message
  - peerd: prepares and sends `Pong` response
3. Local flow
  - peerd: receives `Pong` message and proceeds

## Channel creation
1. Local flow
	- user->cli: `create channel <peer>` command
	- cli->lnpd: `OpenChannelWith`
	- lnpd: launches `channeld` and waits for it's connection
	- channeld->lnpd: `Hello`
	- lnpd: registers channeld
	- lnpd->channeld: `OpenChannelWith`
	- channeld->peerd: `OpenChannel` message
	- peerd: sends remote peer `OpenChannel` message
2. Remote flow
	- peerd: receives `OpenChannel` message
	- peerd->lnpd: forwards `OpenChannel` message
	- lnpd: launches `channeld` and waits for it's connection
	- channeld->lnpd: `Hello`
	- lnpd: registers channeld
	- lnpd->channeld: `AcceptChannelFrom`
	- channeld->peerd: `AcceptChannel` message
	- peerd: sends remote peer `AcceptChannel` message
3. Local flow
	- peerd: receives `AcceptChannel` message
	- peerd->channeld: forwards `AcceptChannel` message
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
	- channeld->peerd: `UpdateAddHtlc` message
  - 
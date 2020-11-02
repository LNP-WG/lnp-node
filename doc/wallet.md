# Wallet

Tasks to cover:
* Key management
	- HD private keys
	- HD public keys
	- Signatures
* Tracking onchain tx
	- Including multisigs (when other party key is known)
	- Including tweaked keys
* Constructing tx
  - Transferring to a destination
  - With a given tx structure (for LN)
* RGB assets
  - Manage stash and parsed list of assets
  - Provide tweaks to existing keys in a given tx outputs


## Init daemon
1. Generate pool master private key
2. It must be manually configured by the user with the list of supported assets

## Create channel
1. Construct tx, transfer from the pool-controlled address to the funding tx with funding mutisig output

## Fund channel with RGB assets
1. Make a consignment from outside of the LNP (like RGB) to the funding output. 
2. Send consignment to the LNP, which must send it to the remote peer as well
3. Verify consignment and add it to LNP-controlled stash + cache
4. Update channel state + update commitment transaction with a tweak
5. Publish last consignment transaction committing the transfer
6. Update commitment tx once again

## 
#!/bin/bash

./target/debug/lnp-cli -d /tmp listen

./target/debug/lnp-cli connect 0275a326e4416600cea2601696e4ae03b239e717e87a290a00dbc1ba4f6df28290@127.0.0.1

./target/debug/lnp-cli propose 0275a326e4416600cea2601696e4ae03b239e717e87a290a00dbc1ba4f6df28290@127.0.0.1 1000 

./target/debug/lnp-cli info

echo -n "Temp. channel id: " 
read tchid

./target/debug/lnp-cli fund "$tchid" 4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b:0


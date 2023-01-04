# Federated and decentralised oracle 

A paraxiom is a statement that appears to contradict an accepted truth, while in reality it is merely an alternate version of the accepted truth. 
 It is often used in philosophical debates to challenge a traditional belief or assumption.
### Builds a parachain with the relay chain's token (In this case ROC)
````
cargo build --release --locked -p polkadot-parachain
````

### Runs a local network with oracle(contracts) parachain
````
./zombienet(-macos) --provider native spawn zombienet/examples/oracle_rococo_local_network.toml
````


### This runs the parachain on Rococo
````
./target/release/polkadot-parachain --chain oracle-rococo
````

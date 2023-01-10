# Federated and decentralised oracle 
Paraxiom is a term used to refer to statements that appear to be self-contradictory or paradoxical in nature, but which actually make sense upon investigation. Examples of paraxioms are the "law of excluded middle" and the "law of non-contradiction".

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

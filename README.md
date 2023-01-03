# Federated and decentralised parachain oracle 
### Builds a parachain with the relay chain's token (In this case ROC)
````
cargo build --release --locked -p polkadot-parachain
````

### Runs a local network with oracle(contracts) parachain
./zombienet(-macos) --provider native spawn zombienet/examples/oracle_rococo_local_network.toml



### This runs the parachain on Rococo
````
./target/release/polkadot-parachain --chain oracle-rococo
````

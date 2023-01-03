# Federated and decentralised parachain oracle 
### Builds a parachain with the relay chain's token (In this case ROC)
````
cargo build --release --locked -p polkadot-parachain
````
### This runs the parachain on Rococo
````
./target/release/polkadot-parachain --chain oracle-rococo
````

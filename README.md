# Federated and decentralised parachain oracle 
### This builds a parachain with the relay chain's token (In this case ROC)
1. cargo build --release --locked -p polkadot-parachain
This runs the realy chain
2. ./target/release/polkadot-parachain --chain oracle-rococo

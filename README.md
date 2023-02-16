
# Federated and decentralized parachain oracle system

This project will offer a multi level data aggregation solution, 
a staking/slashing mechanism, a efficient registration service a
and a just dispute resolution process.
The parachain oracle ecosystem will be built using substrate 
pallets/contracts and xcm.
This protocol, along with the Polkadot and Kusama parachain 
community effort will provide a substantial level of trust.



### Phat Oracle


This project has been mandated to implement Phat offchain rollups,
as a mechanism for securing reporter data.


### running instructions:
##### *** this is MVP work, please forgive the imbricated applications/directories. ***
##### *** Strongly suggested using latest ubuntu ***

##### Build 
```
cargo build --release
```
##### Run relay and parachain 
##### Get your zombienet on:
https://github.com/paritytech/zombienet/releases
```
cd testnet

./zombienet-macos spawn --provider native network.toml

```
##### Possible ubuntu issue: Install openssl
```
wget https://www.openssl.org/source/openssl-1.1.1o.tar.gz
tar -zxvf openssl-1.1.1o.tar.gz
cd openssl-1.1.1o
./config --prefix=$HOME/.local --openssldir=$HOME/.local
make
make test
make install
```
##### Set the env var before running pruntime
```
export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:$HOME/.local/lib"
```

##### Run rollups test
```
cd ~/phat-offchain-rollup/phat

reset && yarn devphase contract test
```
You should you see output on the host parachain.

![Screenshot from 2023-02-16 11-24-43](https://user-images.githubusercontent.com/6019499/219456659-92e82249-ca82-4139-bc35-d63fe0331cec.png)



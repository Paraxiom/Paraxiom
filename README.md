
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
#### Initial boot
##### Build 
```
cargo build --release
```
##### Install polkadot in ~/ directory in ~/ directory
##### Install Paraxiom node or consumer
#### Run relay and parachain 
##### Get your zombienet on:
https://github.com/paritytech/zombienet/releases

```
cd testnet

./zombienet-linux-x64 spawn --provider native network.toml

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
##### Set the env var
```
export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:$HOME/.local/lib"
```

#### Run rollups test
```
cd ~/phat-offchain-rollup/phat

yarn install

reset && yarn devphase contract test
```
Running the test, you should you see activity in polkadot-js, x5.
![Screenshot from 2023-02-24 16-31-08](https://user-images.githubusercontent.com/6019499/221297194-90f63e18-7785-4710-8037-b4e9c457c268.png)
Viewing the Phat Oracle chain state for price feeds, you can conclude these are all different,
all being processed at different timestamps.


![Screenshot from 2023-02-24 16-19-37](https://user-images.githubusercontent.com/6019499/221298235-c23ad6f6-8046-4311-a810-8873cb300287.png)

You can trigger an average for the current storage with the Phat Oracle average extrinsic.
![Screenshot from 2023-02-24 16-31-08](https://user-images.githubusercontent.com/6019499/221298614-eafe73b7-779a-4c45-8614-de22a88ba84e.png)

You can then subsequently view this average with the chain state for Phat Oracle averages. 

![Screenshot from 2023-02-24 16-21-21](https://user-images.githubusercontent.com/6019499/221296456-5ad4be2b-0898-4881-81a1-14688065ec59.png)
 https://github.com/AstarNetwork/swanky-plugin-phala
* https://www.ise.tu-berlin.de/fileadmin/fg308/publications/2018/Off-chaining_Models_and_Approaches_to_Off-chain_Computations.pdf
* https://github.com/l00k/devphase






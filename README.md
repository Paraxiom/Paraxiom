
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

##### Run rollups test
```
cd ~/phat-offchain-rollup/phat

reset && yarn devphase contract test
```
Running the test, you should you see activity in polkadot-js, x5.
![Screenshot from 2023-02-24 16-31-08](https://user-images.githubusercontent.com/6019499/221297194-90f63e18-7785-4710-8037-b4e9c457c268.png)
Viewing the Phat Oracle chain state for price feeds, you can conclude these are all different,
being processed at different timestamps. 
![Screenshot from 2023-02-24 16-21-21](https://user-images.githubusercontent.com/6019499/221296456-5ad4be2b-0898-4881-81a1-14688065ec59.png)

You can trigger an average for the current storage with the Phat Oracle average extrinsic.
![Screenshot from 2023-02-16 11-2![Screenshot from 2023-02-24 16-21-21](https://user-images.githubusercontent.com/6019499/221296327-6d8e3336-9b28-44f6-b07d-5b1950e36c75.png)
4-43](https://user-images.githubusercontent.com/6019499/219456659-92e82249-ca82-4139-bc35-d63fe0331cec.png)



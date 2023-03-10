import { SubPriceFeed } from '@/typings/SubPriceFeed';
import { Sub0Factory } from '@/typings/Sub0Factory'
import * as PhalaSdk from '@phala/sdk';
import { ApiPromise } from '@polkadot/api';
import type { KeyringPair } from '@polkadot/keyring/types';
import { ContractType } from 'devphase';

import 'dotenv/config';
import { LocalScheduler } from '@/typings/LocalScheduler';

async function delay(ms: number): Promise<void> {
    return new Promise( resolve => setTimeout(resolve, ms) );
}

describe('Substrate Offchain Rollup', () => {
    //get this value from Oracle node
    const httpRpc: string =  "http://127.0.0.1:30489";
    const secretBob: string = "0x398f0c28f98885e046333d4a41c19cee4c37368a9832c6502f6cfd182e2aef89";

    let priceFeedFactory: SubPriceFeed.Factory;
    let priceFeed: SubPriceFeed.Contract;
    let priceFeed2: SubPriceFeed.Contract;
    let priceFeed3: SubPriceFeed.Contract;
    let priceFeed4: SubPriceFeed.Contract;
    let priceFeed5: SubPriceFeed.Contract;
    let priceFeed6: SubPriceFeed.Contract;
    
    let priceFeedCodeHash: string;
    let sub0Factory: Sub0Factory.Factory;
    let sub0: Sub0Factory.Contract;
    let api: ApiPromise;
    let alice : KeyringPair;
    let certAlice : PhalaSdk.CertificateData;
    const txConf = { gasLimit: "10000000000000", storageDepositLimit: null };
    

    before(async function() {
        priceFeedFactory = await this.devPhase.getFactory('sub_price_feed');
        sub0Factory = await this.devPhase.getFactory('sub0_factory');
        priceFeedCodeHash = priceFeedFactory.metadata.source.hash;
        await priceFeedFactory.deploy();
        await sub0Factory.deploy();
        expect(priceFeedCodeHash.startsWith('0x')).to.be.true; 
        api = this.api;
        alice = this.devPhase.accounts.alice;
        certAlice = await PhalaSdk.signCertificate({
            api,
            pair: alice,
        });
        console.log('Signer:', alice.address.toString());
        console.log('PriceFeed code:', priceFeedCodeHash)
    });

    describe('SubPriceFeed', () => {
        before(async function() {
            this.timeout(100_000);
            // Deploy contract
            priceFeed = await priceFeedFactory.instantiate('default', [], {transferToCluster: 1e12});
            await delay(1000);
            priceFeed2 = await priceFeedFactory.instantiate('default', [], {transferToCluster: 1e12});
            await delay(1000);
            priceFeed3 = await priceFeedFactory.instantiate('default', [], {transferToCluster: 1e12});
            await delay(1000);
            priceFeed4 = await priceFeedFactory.instantiate('default', [], {transferToCluster: 1e12});
            await delay(1000);
            priceFeed5 = await priceFeedFactory.instantiate('default', [], {transferToCluster: 1e12});
            await delay(1000);
            priceFeed6 = await priceFeedFactory.instantiate('default', [], {transferToCluster: 1e12});
            await delay(1000);
            console.log('SubPriceFeed deployed at', priceFeed.address.toString());
        });

        // it('should have correct owners', async function() {
        //     const feedOwner = await priceFeed.query.owner(certAlice, {});
        //     expect(feedOwner.result.isOk).to.be.true;
        //     expect(feedOwner.output.toString()).to.be.equal(alice.address.toString());
        // });

        it('should be configurable', async function() {
            // Config the oracle
            await delay(1000);
            const feedConfig = await priceFeed.tx
                .config(txConf, httpRpc, 100, secretBob as any, 'polkadot', 'usd')
                .signAndSend(alice, {nonce: -1});
            await delay(1000);
            const feedConfig2 = await priceFeed2.tx
            .config(txConf, httpRpc, 100, secretBob as any, 'polkadot', 'usd')
            .signAndSend(alice, {nonce: -1});
            await delay(1000);
            const feedConfig3 = await priceFeed3.tx
            .config(txConf, httpRpc, 100, secretBob as any, 'polkadot', 'usd')
            .signAndSend(alice, {nonce: -1});
            await delay(1000);
            const feedConfig4 = await priceFeed4.tx
            .config(txConf, httpRpc, 100, secretBob as any, 'polkadot', 'usd')
            .signAndSend(alice, {nonce: -1});
            await delay(1000);
            const feedConfig5 = await priceFeed5.tx
            .config(txConf, httpRpc, 100, secretBob as any, 'polkadot', 'usd')
            .signAndSend(alice, {nonce: -1});
            await delay(1000);    
            const feedConfig6 = await priceFeed6.tx
            .config(txConf, httpRpc, 100, secretBob as any, 'polkadot', 'usd')
            .signAndSend(alice, {nonce: -1});
            await delay(1000);    

        });  
            
        it('should init rollups', async function() {
                
            // Init the rollup on the blockchain
            const init = await priceFeed.query.maybeInitRollup(certAlice, {});
            await delay(1000);    
            const init2 = await priceFeed2.query.maybeInitRollup(certAlice, {});
            await delay(1000);    
            const init3 = await priceFeed3.query.maybeInitRollup(certAlice, {});
            await delay(1000);    
            const init4 = await priceFeed4.query.maybeInitRollup(certAlice, {});
            await delay(1000);    
            const init5 = await priceFeed5.query.maybeInitRollup(certAlice, {});
            await delay(1000);    
            const init6 = await priceFeed6.query.maybeInitRollup(certAlice, {});
            await delay(1000);    
            
            // expect(init.result.isOk).to.be.true;
            // expect(init.output.isOk).to.be.true;
            // expect(init.output.asOk.isSome).to.be.true;
        });

        it('can submit tx', async function() {
            // this.timeout(1000*30_000);

            const feed = await priceFeed.query.feedPrice(certAlice, {});
            await delay(1000);
            const feed2 = await priceFeed2.query.feedPrice(certAlice, {});
            await delay(1000);
            const feed3 = await priceFeed3.query.feedPrice(certAlice, {});
            await delay(1000);
            const feed4 = await priceFeed4.query.feedPrice(certAlice, {});
            await delay(1000);
            const feed5 = await priceFeed5.query.feedPrice(certAlice, {});
            await delay(1000);
            const feed6 = await priceFeed.query.feedPrice(certAlice, {});
            

            // // expect(feed.result.isOk).to.be.true;
            // expect(feed.output.isOk).to.be.true;
            // expect(feed.output.asOk.isSome).to.be.true;
            // await delay(3*1000);

            // The response should be received on the blockchain
            // const receivedPrice = await api.query.phatOracle.priceFeeds.entries(alice.address);
            // expect(receivedPrice.length).to.be.equal(1);
        });
    });

    // describe('Sub0Factory', () => {
    //     before(async function() {
    //         this.timeout(30_000);
    //         // Deploy contract
    //         sub0 = await sub0Factory.instantiate('default', [], {transferToCluster: 1e12});
    //         console.log('Sub0Factory deployed at', sub0.address.toString());
    //     });

    //     it('should has correct owners', async function() {
    //         const sub0Owner = await sub0.query.owner(certAlice, {});
    //         expect(sub0Owner.result.isOk).to.be.true;
    //         expect(sub0Owner.output.toString()).to.be.equal(alice.address.toString());
    //     });

    //     it('can be configured', async function() {
    //         // Config the oracle
    //         const sub0Config = await sub0.tx
    //             .config(txConf, httpRpc, 100, secretBob as any, priceFeedCodeHash)
    //             .signAndSend(alice, {nonce: -1});
    //         console.log('Sub0Factory configured', sub0Config.toHuman());
    //         await delay(4*1000);

    //         const config = await sub0.query.getConfig(certAlice, {})
    //         // expect(config.result.isOk).to.be.true;
    //         // expect(config.output.isOk).to.be.true;
    //         // expect(config.output.asOk.length).to.be.equal(2);
    //     });

    //     let priceFeed1: SubPriceFeed.Contract;
    //     it('can deploy price feeds', async function() {
    //         let deploy = await api.tx.utility.batchAll([
    //             sub0.tx.deployPriceFeed(txConf, 'feed1', 'bitcpoin', 'usd'),
    //             sub0.tx.deployPriceFeed(txConf, 'feed2', 'polkadot', 'usd'),
    //         ]).signAndSend(alice, {nonce: -1});

    //         console.log('PriceFeed1&2 deployed', deploy.toHuman());
    //         await delay(5*1000);

    //         let deployments = await sub0.query.getDeployments(certAlice, {});
    //         expect(deployments.result.isOk).to.be.true;
    //         expect(deployments.output.asOk.length).to.be.equal(2);

    //         // Get the address in hex, and attach to it.
    //         //
    //         // Note that `contractId.toString()` returns and SS58 encoded address by default, but
    //         // Polkadot.js cannot parse it to H256.
    //         let feed1Addr = deployments.output.asOk[1].contractId.toHex();
    //         priceFeed1 = await priceFeedFactory.attach(feed1Addr);
    //     });

    //     it('can trigger a rollup', async function() {
    //         // Init the rollup on the blockchain
    //         const init = await priceFeed1.query.maybeInitRollup(certAlice, {});
    //         // expect(init.result.isOk).to.be.true;
    //         // expect(init.output.isOk).to.be.true;
    //         // expect(init.output.asOk.isSome).to.be.true;
    //         await delay(3*1000);

    //         // Trigger a rollup
    //         const feed = await priceFeed1.query.feedPrice(certAlice, {});
    //         // expect(feed.result.isOk).to.be.true;
    //         // expect(feed.output.isOk).to.be.true;
    //         // expect(feed.output.asOk.isSome).to.be.true;
    //         await delay(3*1000);

    //         // The response should be received on the blockchain
    //         // const receivedPrice = await api.query.phatOracle.priceFeeds.entries(alice.address);
    //         // expect(receivedPrice.length).to.be.equal(2);  // 2 in totoal: 1 existing & 1 more
    //     });

    // });

    // // To keep the blockchain running after the test, remove the "skip" in the following test
    // after('keeps running', async function() {
    //     this.timeout(1000 * 30_000);
    //     await delay(1000 * 30_000);
    // });
});
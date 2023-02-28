const fs = require('fs');
const path = require('path');
const { ApiPromise, Keyring } = require('@polkadot/api');
const { WsProvider } = require('@polkadot/rpc-provider');

const SCHEMA_PATH = path.join(__dirname, './', 'schema.json');
const EXTRINSIC_DATA_PATH = path.join(__dirname, './', 'extrinsic_block.json');
const START_BLOCK = process.env.START_BLOCK ? parseInt(process.env.START_BLOCK, 10) : 0
const END_BLOCK = process.env.END_BLOCK ? parseInt(process.env.END_BLOCK, 10) : 10
const WS_URL = process.env.WS_URL || 'ws://127.0.0.1:8844'


async function main() {
    const provider = new WsProvider(WS_URL)

    let api;
    if (!fs.existsSync(SCHEMA_PATH)) {
        console.log('Custom Schema missing, using default schema.');
        api = await ApiPromise.create({ provider });
    } else {
        const { types, rpc } = JSON.parse(fs.readFileSync(SCHEMA_PATH, 'utf8'));
        api = await ApiPromise.create({
            provider,
            types,
            rpc,
        });
    } 

    // const signer = new Keyring({ type: 'sr25519' }).addFromUri(
    //     `${process.env.ACCOUNT_KEY || '//Alice'}`
    // )

    let allEntries = await api.query.registryPallet.apiFeeds.entries()
    //console.log(`apiFeeds query: ${allEntries}`)

    allEntries.forEach(([{ args: [account, key] }, value]) => {
        console.log(`${account}: ${key.toString()}, feed: ${value.toString()}`);
    });

}

main().catch(console.error).finally(() => process.exit());

// To run with node.js, first: $ yarn add $polkadot/api
// then add commented lines:
const { ApiPromise, WsProvider } = require('@polkadot/api');

const api_endpoint = 'ws://127.0.0.1:8844'

async function main () {
	const api = await ApiPromise.create({
        provider: new WsProvider(api_endpoint)
    });
	const metadata = await api.rpc.state.getMetadata();

	console.log(JSON.stringify(metadata.asLatest.toHuman(), null, 2));
}

main().catch(console.error);


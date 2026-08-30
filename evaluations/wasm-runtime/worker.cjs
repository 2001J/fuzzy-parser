const { parentPort, workerData } = require('node:worker_threads');
const signal = new Int32Array(workerData.signal);
globalThis.__fuzzyParserParseEntry = () => Atomics.store(signal, 0, 1);
const wasm = require(workerData.pkg);
const bytes = Buffer.from('name,count,enabled\nsample,42,true\n'.repeat(25000));
parentPort.postMessage('ready');
wasm.parse_bytes_json('csv', bytes, 'cancel.csv', workerData.schema);
parentPort.postMessage('complete');

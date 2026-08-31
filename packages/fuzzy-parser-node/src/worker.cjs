'use strict';

const { createHash } = require('node:crypto');
const { readFileSync } = require('node:fs');
const { parentPort } = require('node:worker_threads');

const PROTOCOL_VERSION = 1;
const IDENTITY_PATH = require.resolve('./runtime/identity.json');
const GLUE_PATH = require.resolve('./runtime/parser_wasm.cjs');
const WASM_PATH = require.resolve('./runtime/parser_wasm_bg.wasm');
let currentRequestId;

function sha256(path) {
  return createHash('sha256').update(readFileSync(path)).digest('hex');
}

function fail(code, message) {
  parentPort.postMessage({ type: 'adapter_error', code, message });
  parentPort.close();
}

function loadRuntime() {
  const identity = JSON.parse(readFileSync(IDENTITY_PATH, 'utf8'));
  if (
    identity.adapterName !== '@fuzzy-parser/node' ||
    identity.adapterVersion !== '0.1.0' ||
    identity.contractVersion !== '0.1' ||
    identity.parserVersion !== '0.1.0' ||
    identity.schemaVersion !== '0.1' ||
    identity.wasmBindgenVersion !== '0.2.115' ||
    identity.assets?.glue?.file !== 'parser_wasm.cjs' ||
    identity.assets?.glue?.sha256 !== sha256(GLUE_PATH) ||
    identity.assets?.wasm?.sha256 !== sha256(WASM_PATH)
  ) {
    throw new Error('runtime asset identity mismatch');
  }
  globalThis.__fuzzyParserParseEntry = () => {
    parentPort.postMessage({ type: 'entered', requestId: currentRequestId });
  };
  const runtime = require(GLUE_PATH);
  const compiled = JSON.parse(runtime.runtime_identity_json());
  const expected = {
    adapterVersion: identity.adapterVersion,
    contractVersion: identity.contractVersion,
    parserVersion: identity.parserVersion,
    schemaVersion: identity.schemaVersion,
    sourceIdentity: identity.sourceIdentity,
    wasmBindgenVersion: identity.wasmBindgenVersion,
  };
  if (JSON.stringify(compiled) !== JSON.stringify(expected)) {
    throw new Error('compiled runtime identity mismatch');
  }
  return runtime;
}

let runtime;
try {
  runtime = loadRuntime();
  parentPort.postMessage({ type: 'ready', protocolVersion: PROTOCOL_VERSION });
} catch {
  fail('INITIALIZATION_FAILED', 'parser runtime assets are missing, corrupt, or incompatible');
}

if (runtime) {
  parentPort.once('message', (message) => {
    if (
      !message ||
      message.type !== 'parse' ||
      message.protocolVersion !== PROTOCOL_VERSION ||
      !Number.isSafeInteger(message.requestId) ||
      typeof message.format !== 'string' ||
      !(message.bytes instanceof Uint8Array) ||
      typeof message.schema !== 'string' ||
      (message.filename !== undefined && typeof message.filename !== 'string') ||
      (message.options !== undefined && typeof message.options !== 'string')
    ) {
      fail('PROTOCOL_ERROR', 'parser Worker received a malformed request');
      return;
    }
    currentRequestId = message.requestId;
    let envelope;
    try {
      envelope = JSON.parse(
        runtime.parse_bytes_json(
          message.format,
          message.bytes,
          message.filename,
          message.schema,
          message.options,
        ),
      );
    } catch {
      fail('PROTOCOL_ERROR', 'parser runtime returned a malformed envelope');
      return;
    }
    if (!['success', 'parser_failure'].includes(envelope?.kind) || typeof envelope.json !== 'string') {
      fail('PROTOCOL_ERROR', 'parser runtime returned an incompatible envelope');
      return;
    }
    parentPort.postMessage({
      type: 'result',
      requestId: message.requestId,
      outcome: envelope.kind,
      json: envelope.json,
    });
    parentPort.close();
  });
}

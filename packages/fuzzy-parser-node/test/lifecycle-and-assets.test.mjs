import assert from 'node:assert/strict';
import { Worker } from 'node:worker_threads';
import { cp, mkdtemp, readFile, rm, unlink, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import test from 'node:test';
import { loadCjs, packageRoot, publicRequest, schemas } from './helpers.mjs';

const api = loadCjs();
const runtime = api.__testing ?? (await import('node:module')).createRequire(import.meta.url)(join(packageRoot, 'dist/runtime.cjs'));

async function waitForMessage(worker, predicate) {
  return new Promise((resolve, reject) => {
    const onMessage = (message) => {
      if (!predicate(message)) return;
      worker.off('error', reject);
      resolve(message);
    };
    worker.on('message', onMessage);
    worker.once('error', reject);
  });
}

test('malformed Worker messages fail with the bounded adapter protocol error', async () => {
  const worker = new Worker(join(packageRoot, 'dist/worker.cjs'));
  await waitForMessage(worker, (message) => message?.type === 'ready');
  worker.postMessage({ type: 'parse', protocolVersion: 999 });
  const failure = await waitForMessage(worker, (message) => message?.type === 'adapter_error');
  assert.deepEqual(
    { code: failure.code, message: failure.message },
    { code: 'PROTOCOL_ERROR', message: 'parser Worker received a malformed request' },
  );
  await worker.terminate();
});

test('AbortSignal terminates and reaps a Worker after actual parser entry, with no partial result', async () => {
  const controller = new AbortController();
  let entered = false;
  runtime.__testing.setWorkerObserver((message) => {
    if (message?.type === 'entered') {
      entered = true;
      controller.abort();
    }
  });
  const bytes = Buffer.from(`name,count,enabled\n${'sample,42,true\n'.repeat(50_000)}`);
  await assert.rejects(
    runtime.parse(publicRequest('csv', bytes, schemas['inventory-supported'], 'abort.csv'), { signal: controller.signal, timeoutMs: 120_000 }),
    (error) => error.code === 'ABORTED',
  );
  runtime.__testing.setWorkerObserver(undefined);
  assert.equal(entered, true);
  assert.equal(runtime.__testing.activeWorkerCount(), 0);
});

test('deadline terminates synchronous WASM work and a subsequent call recovers', async () => {
  let entered = false;
  runtime.__testing.setWorkerObserver((message) => {
    if (message?.type === 'entered') entered = true;
  });
  const bytes = Buffer.from(`name,count,enabled\n${'sample,42,true\n'.repeat(90_000)}`);
  await assert.rejects(
    runtime.parse(publicRequest('csv', bytes, schemas['inventory-supported'], 'timeout.csv'), { timeoutMs: 100 }),
    (error) => error.code === 'TIMEOUT',
  );
  runtime.__testing.setWorkerObserver(undefined);
  assert.equal(entered, true, 'deadline fired after core parser entry');
  assert.equal(runtime.__testing.activeWorkerCount(), 0);
  const recovered = await runtime.parse(publicRequest('text', Buffer.from('42')));
  assert.equal(recovered.contract_version, '0.1');
});

test('concurrent calls use isolated Workers and all are reaped', async () => {
  let maximum = 0;
  runtime.__testing.setWorkerObserver(() => {
    maximum = Math.max(maximum, runtime.__testing.activeWorkerCount());
  });
  const responses = await Promise.all(
    Array.from({ length: 4 }, (_, index) => runtime.parse(publicRequest('text', Buffer.from(`value ${index + 1}`)))),
  );
  runtime.__testing.setWorkerObserver(undefined);
  assert.equal(responses.length, 4);
  assert(maximum >= 2, 'at least two per-call Workers overlapped');
  assert.equal(runtime.__testing.activeWorkerCount(), 0);
});

for (const mutation of ['missing', 'corrupt', 'wrong-version']) {
  test(`${mutation} runtime assets fail initialization without parser fallback`, async () => {
    const directory = await mkdtemp(join(tmpdir(), 'fuzzy-parser-assets-'));
    const copy = join(directory, 'package');
    await cp(packageRoot, copy, { recursive: true, filter: (source) => !source.includes('/node_modules') && !source.includes('/wasm/target') && !source.includes('/.toolchain') });
    const wasm = join(copy, 'dist/runtime/parser_wasm_bg.wasm');
    const identityPath = join(copy, 'dist/runtime/identity.json');
    if (mutation === 'missing') await unlink(wasm);
    if (mutation === 'corrupt') await writeFile(wasm, Buffer.from('not wasm'));
    if (mutation === 'wrong-version') {
      const identity = JSON.parse(await readFile(identityPath, 'utf8'));
      identity.adapterVersion = '999.0.0';
      await writeFile(identityPath, JSON.stringify(identity));
    }
    try {
      const copied = (await import('node:module')).createRequire(import.meta.url)(join(copy, 'dist/index.cjs'));
      await assert.rejects(
        copied.parse(publicRequest('text', Buffer.from('42'))),
        (error) => error.code === 'INITIALIZATION_FAILED',
      );
    } finally {
      await rm(directory, { recursive: true, force: true });
    }
  });
}

import assert from 'node:assert/strict';
import { spawn, spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { cp, mkdtemp, mkdir, readFile, readdir, rm, stat, writeFile } from 'node:fs/promises';
import { createServer } from 'node:net';
import { dirname, join, resolve } from 'node:path';
import { tmpdir } from 'node:os';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const packageRoot = join(root, 'packages/fuzzy-parser-node');
const bindgen = join(packageRoot, '.toolchain/bin/wasm-bindgen');
const nextFixture = join(packageRoot, 'test/fixtures/next');
const temporary = await mkdtemp(join(tmpdir(), 'fuzzy-parser-package-'));

function run(command, args, options = {}) {
  process.stdout.write(`+ ${[command, ...args].join(' ')}\n`);
  const result = spawnSync(command, args, { cwd: packageRoot, stdio: 'inherit', ...options });
  if (result.status !== 0) process.exit(result.status ?? 1);
}

function capture(command, args, options = {}) {
  process.stdout.write(`+ ${[command, ...args].join(' ')}\n`);
  const result = spawnSync(command, args, { cwd: packageRoot, encoding: 'utf8', ...options });
  if (result.status !== 0) {
    process.stderr.write(result.stderr ?? '');
    process.exit(result.status ?? 1);
  }
  return result.stdout;
}

async function availablePort() {
  const server = createServer();
  await new Promise((resolvePromise, reject) => server.listen(0, '127.0.0.1', resolvePromise).once('error', reject));
  const address = server.address();
  await new Promise((resolvePromise, reject) => server.close((error) => error ? reject(error) : resolvePromise()));
  return address.port;
}

async function waitFor(url, child, timeoutMs) {
  const deadline = Date.now() + timeoutMs;
  let lastError;
  while (Date.now() < deadline) {
    if (child.exitCode !== null) throw new Error(`Next server exited early with ${child.exitCode}`);
    try {
      const response = await fetch(url);
      if (response.ok) return response;
      lastError = new Error(`HTTP ${response.status}`);
    } catch (error) {
      lastError = error;
    }
    await new Promise((resolvePromise) => setTimeout(resolvePromise, 100));
  }
  throw lastError ?? new Error('Next server did not become ready');
}

async function assertNoConsumerName(path) {
  for (const entry of await readdir(path, { withFileTypes: true })) {
    const full = join(path, entry.name);
    if (entry.isDirectory()) await assertNoConsumerName(full);
    else if (/\.(?:cjs|mjs|js|json|d\.ts|md)$/.test(entry.name)) {
      const content = await readFile(full, 'utf8');
      assert(!/QualEvents/i.test(content), `${full} must remain consumer-neutral`);
    }
  }
}

try {
  assert.deepEqual(process.argv.slice(2), [], 'verify-node-package.mjs takes no arguments');
  run('npm', ['ci', '--ignore-scripts']);
  const version = spawnSync(bindgen, ['--version'], { encoding: 'utf8' });
  if (version.status !== 0 || version.stdout.trim() !== 'wasm-bindgen 0.2.115') {
    run('cargo', ['install', 'wasm-bindgen-cli', '--version', '0.2.115', '--locked', '--root', '.toolchain']);
  }
  run('npm', ['run', 'build']);
  run('npm', ['run', 'lint']);
  run('npm', ['run', 'typecheck']);
  run('npm', ['test']);

  const packDirectory = join(temporary, 'pack');
  await mkdir(packDirectory);
  const packed = JSON.parse(capture('npm', ['pack', '--ignore-scripts', '--json', '--pack-destination', packDirectory]));
  assert.equal(packed.length, 1);
  const tarball = join(packDirectory, packed[0].filename);
  const entries = capture('tar', ['-tzf', tarball], { cwd: root }).trim().split('\n');
  for (const required of [
    'package/dist/index.cjs',
    'package/dist/index.mjs',
    'package/dist/index.d.ts',
    'package/dist/worker.cjs',
    'package/dist/runtime/identity.json',
    'package/dist/runtime/parser_wasm.cjs',
    'package/dist/runtime/parser_wasm_bg.wasm',
  ]) assert(entries.includes(required), `packed package retains ${required}`);

  const nodeConsumer = join(temporary, 'node-consumer');
  await mkdir(nodeConsumer);
  run('npm', ['init', '-y'], { cwd: nodeConsumer });
  run('npm', ['install', '--ignore-scripts', tarball], { cwd: nodeConsumer });
  const consumerSource = `
const assert = require('node:assert/strict');
const cjs = require('@fuzzy-parser/node');
const schema = {schema_version:'0.1',record_name:'generic',fields:[],options:{allow_unknown_fields:true}};
(async () => {
  const esm = await import('@fuzzy-parser/node');
  for (const api of [cjs, esm]) {
    const result = await api.parse({input:{format:'text',bytes:Buffer.from('raw 42')},schema});
    assert.equal(result.contract_version, '0.1');
    assert.equal(result.source_evidence.document.blocks[0].value.value, 'raw 42');
  }
})().catch((error) => { console.error(error); process.exitCode = 1; });
`;
  const consumerPath = join(nodeConsumer, 'consumer.cjs');
  await writeFile(consumerPath, consumerSource);
  run(process.execPath, [consumerPath], { cwd: nodeConsumer, env: { PATH: '/usr/bin:/bin' } });
  await assertNoConsumerName(join(nodeConsumer, 'node_modules/@fuzzy-parser/node'));

  const nextConsumer = join(temporary, 'next-consumer');
  await cp(nextFixture, nextConsumer, { recursive: true });
  await writeFile(join(nextConsumer, 'package.json'), `${JSON.stringify({
    name: 'generic-fuzzy-parser-next-fixture',
    private: true,
    version: '0.0.0',
    scripts: { build: 'next build' },
    dependencies: {
      '@fuzzy-parser/node': `file:${tarball}`,
      next: '16.3.3',
      react: '19.2.8',
      'react-dom': '19.2.8',
    },
  }, null, 2)}\n`);
  run('npm', ['install', '--ignore-scripts'], { cwd: nextConsumer });
  run('npm', ['run', 'build'], { cwd: nextConsumer, env: { ...process.env, NEXT_TELEMETRY_DISABLED: '1' } });

  const standalonePackage = join(nextConsumer, '.next/standalone/node_modules/@fuzzy-parser/node/dist');
  for (const asset of ['worker.cjs', 'runtime/identity.json', 'runtime/parser_wasm.cjs', 'runtime/parser_wasm_bg.wasm']) {
    await stat(join(standalonePackage, asset));
  }
  const identity = JSON.parse(await readFile(join(standalonePackage, 'runtime/identity.json'), 'utf8'));
  const wasm = await readFile(join(standalonePackage, 'runtime/parser_wasm_bg.wasm'));
  assert.equal(createHash('sha256').update(wasm).digest('hex'), identity.assets.wasm.sha256);

  const port = await availablePort();
  const child = spawn(process.execPath, ['.next/standalone/server.js'], {
    cwd: nextConsumer,
    env: { ...process.env, HOSTNAME: '127.0.0.1', PORT: String(port), NEXT_TELEMETRY_DISABLED: '1' },
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  let stdout = '';
  let stderr = '';
  child.stdout.on('data', (chunk) => { if (stdout.length < 64 * 1024) stdout += chunk; });
  child.stderr.on('data', (chunk) => { if (stderr.length < 64 * 1024) stderr += chunk; });
  try {
    const response = await waitFor(`http://127.0.0.1:${port}/api/parse`, child, 30_000);
    assert.deepEqual(await response.json(), {
      contractVersion: '0.1',
      parserVersion: '0.1.0',
      recordName: 'inventory_item',
      raw: '42',
    });
  } catch (error) {
    process.stderr.write(stdout);
    process.stderr.write(stderr);
    throw error;
  } finally {
    child.kill('SIGTERM');
    await new Promise((resolvePromise) => child.once('exit', resolvePromise));
  }

  process.stdout.write(`${JSON.stringify({
    package: '@fuzzy-parser/node@0.1.0',
    node: process.version,
    os: process.platform,
    architecture: process.arch,
    next: '16.3.3',
    react: '19.2.8',
    packedBytes: (await stat(tarball)).size,
    wasmBytes: identity.assets.wasm.bytes,
    wasmSha256: identity.assets.wasm.sha256,
    sourceIdentity: identity.sourceIdentity,
  }, null, 2)}\n`);
} finally {
  await rm(temporary, { recursive: true, force: true });
}

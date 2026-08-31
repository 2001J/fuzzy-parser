import { createHash } from 'node:crypto';
import { cp, mkdir, readFile, readdir, rm, stat, writeFile } from 'node:fs/promises';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const repositoryRoot = resolve(packageRoot, '../..');
const dist = join(packageRoot, 'dist');
const generated = join(packageRoot, '.generated');
const runtimeDir = join(dist, 'runtime');
const bindgen = join(packageRoot, '.toolchain/bin/wasm-bindgen');

function run(command, args, options = {}) {
  const result = spawnSync(command, args, { cwd: packageRoot, stdio: 'inherit', ...options });
  if (result.status !== 0) process.exit(result.status ?? 1);
}

async function filesUnder(path) {
  const entries = await readdir(path, { withFileTypes: true });
  const paths = [];
  for (const entry of entries.sort((left, right) => left.name.localeCompare(right.name))) {
    const full = join(path, entry.name);
    if (entry.isDirectory()) paths.push(...await filesUnder(full));
    else paths.push(full);
  }
  return paths;
}

async function digestFiles(paths) {
  const hash = createHash('sha256');
  for (const path of paths.sort()) {
    hash.update(relative(repositoryRoot, path));
    hash.update('\0');
    hash.update(await readFile(path));
    hash.update('\0');
  }
  return hash.digest('hex');
}

try {
  await stat(bindgen);
} catch {
  throw new Error('missing local wasm-bindgen 0.2.115; run tools/ci/verify-node-package.mjs from the repository root');
}

const sourceFiles = [
  join(repositoryRoot, 'Cargo.lock'),
  join(repositoryRoot, 'Cargo.toml'),
  join(packageRoot, 'wasm/Cargo.lock'),
  join(packageRoot, 'wasm/Cargo.toml'),
  ...(await filesUnder(join(repositoryRoot, 'crates/parser-core/src'))),
  ...(await filesUnder(join(repositoryRoot, 'crates/parser-formats/src'))),
  ...(await filesUnder(join(repositoryRoot, 'crates/parser-schema/src'))),
  ...(await filesUnder(join(packageRoot, 'wasm/src'))),
];
const sourceIdentity = await digestFiles(sourceFiles);

await rm(dist, { recursive: true, force: true });
await rm(generated, { recursive: true, force: true });
await mkdir(runtimeDir, { recursive: true });
await mkdir(generated, { recursive: true });

run('cargo', [
  'build', '--manifest-path', 'wasm/Cargo.toml', '--release', '--locked',
  '--target', 'wasm32-unknown-unknown',
], { env: { ...process.env, FUZZY_PARSER_SOURCE_IDENTITY: sourceIdentity } });
run(bindgen, [
  '--target', 'nodejs',
  '--out-dir', generated,
  '--out-name', 'parser_wasm',
  'wasm/target/wasm32-unknown-unknown/release/fuzzy_parser_node_wasm.wasm',
]);

for (const file of ['index.cjs', 'index.mjs', 'index.d.ts', 'runtime.cjs', 'worker.cjs']) {
  await cp(join(packageRoot, 'src', file), join(dist, file));
}
await cp(join(generated, 'parser_wasm.js'), join(runtimeDir, 'parser_wasm.cjs'));
await cp(join(generated, 'parser_wasm_bg.wasm'), join(runtimeDir, 'parser_wasm_bg.wasm'));
await cp(join(repositoryRoot, 'LICENSE'), join(packageRoot, 'LICENSE'));

const sha256 = async (path) => createHash('sha256').update(await readFile(path)).digest('hex');
const gluePath = join(runtimeDir, 'parser_wasm.cjs');
const wasmPath = join(runtimeDir, 'parser_wasm_bg.wasm');
const identity = {
  adapterName: '@fuzzy-parser/node',
  adapterVersion: '0.1.0',
  contractVersion: '0.1',
  parserVersion: '0.1.0',
  schemaVersion: '0.1',
  sourceIdentity,
  wasmBindgenVersion: '0.2.115',
  assets: {
    glue: { file: 'parser_wasm.cjs', sha256: await sha256(gluePath), bytes: (await stat(gluePath)).size },
    wasm: { file: 'parser_wasm_bg.wasm', sha256: await sha256(wasmPath), bytes: (await stat(wasmPath)).size },
  },
};
await writeFile(join(runtimeDir, 'identity.json'), `${JSON.stringify(identity, null, 2)}\n`);
await rm(generated, { recursive: true, force: true });

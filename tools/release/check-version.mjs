import { readFile } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const workspacePackages = [
  'parser-api',
  'parser-cli',
  'parser-core',
  'parser-formats',
  'parser-schema',
];
const semver = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?$/;

function capture(text, expression, label) {
  const value = text.match(expression)?.[1];
  if (!value) throw new Error(`could not read ${label}`);
  return value;
}

function lockVersions(lockfile) {
  const versions = new Map();
  for (const section of lockfile.split('[[package]]').slice(1)) {
    const name = section.match(/^\s*name = "([^"]+)"/m)?.[1];
    const version = section.match(/^\s*version = "([^"]+)"/m)?.[1];
    if (name && version && workspacePackages.includes(name)) versions.set(name, version);
  }
  return versions;
}

export async function readVersionState(root = repositoryRoot) {
  const [workspaceManifest, lockfile, nodeManifestText, wasmManifest] = await Promise.all([
    readFile(resolve(root, 'Cargo.toml'), 'utf8'),
    readFile(resolve(root, 'Cargo.lock'), 'utf8'),
    readFile(resolve(root, 'packages/fuzzy-parser-node/package.json'), 'utf8'),
    readFile(resolve(root, 'packages/fuzzy-parser-node/wasm/Cargo.toml'), 'utf8'),
  ]);
  return {
    workspace: capture(workspaceManifest, /\[workspace\.package\][\s\S]*?^version = "([^"]+)"/m, 'workspace package version'),
    node: JSON.parse(nodeManifestText).version,
    wasm: capture(wasmManifest, /\[package\][\s\S]*?^version = "([^"]+)"/m, 'Node WASM crate version'),
    lock: lockVersions(lockfile),
  };
}

export function validateVersionState(state, options = {}) {
  const requested = options.requestedVersion || state.workspace;
  if (!semver.test(requested)) throw new Error(`release version is not semantic versioning: ${requested}`);
  const mismatches = [];
  if (state.workspace !== requested) mismatches.push(`workspace=${state.workspace}`);
  if (state.node !== requested) mismatches.push(`node=${state.node}`);
  if (state.wasm !== requested) mismatches.push(`node-wasm=${state.wasm}`);
  for (const name of workspacePackages) {
    const value = state.lock.get(name);
    if (value !== requested) mismatches.push(`Cargo.lock:${name}=${value ?? 'missing'}`);
  }
  if (mismatches.length > 0) {
    throw new Error(`release version ${requested} is not aligned: ${mismatches.join(', ')}`);
  }
  if (options.refType === 'tag' && options.refName !== `v${requested}`) {
    throw new Error(`release tag must be v${requested}, received ${options.refName}`);
  }
  if (options.publishRequested && options.refName !== 'main') {
    throw new Error('publication is allowed only from main');
  }
  return requested;
}

export async function checkVersion(options = {}) {
  const state = await readVersionState(options.root);
  const version = validateVersionState(state, options);
  return {
    version,
    tag: `v${version}`,
    packages: [...workspacePackages, '@fuzzy-parser/node', 'fuzzy-parser-node-wasm'],
  };
}

const isMain = process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href;
if (isMain) {
  try {
    const summary = await checkVersion({
      requestedVersion: process.argv[2],
      refName: process.env.RELEASE_REF_NAME ?? process.env.GITHUB_REF_NAME,
      refType: process.env.GITHUB_REF_TYPE,
      publishRequested: process.env.RELEASE_PUBLISH_REQUESTED === 'true',
    });
    process.stdout.write(`${JSON.stringify(summary, null, 2)}\n`);
  } catch (error) {
    process.stderr.write(`version check failed: ${error.message}\n`);
    process.exitCode = 1;
  }
}

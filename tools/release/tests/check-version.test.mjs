import assert from 'node:assert/strict';
import test from 'node:test';
import { checkVersion, validateVersionState } from '../check-version.mjs';

const aligned = () => ({
  workspace: '0.1.0',
  node: '0.1.0',
  wasm: '0.1.0',
  lock: new Map([
    ['parser-api', '0.1.0'],
    ['parser-cli', '0.1.0'],
    ['parser-core', '0.1.0'],
    ['parser-formats', '0.1.0'],
    ['parser-schema', '0.1.0'],
  ]),
});

test('the checked-in release surfaces share one package version', async () => {
  const result = await checkVersion();
  assert.equal(result.version, '0.1.0');
  assert.equal(result.tag, 'v0.1.0');
});

test('mismatched package, lock, tag, and publication branch states fail', () => {
  assert.throws(
    () => validateVersionState({ ...aligned(), node: '0.2.0' }),
    /node=0\.2\.0/,
  );
  const missingLock = aligned();
  missingLock.lock.delete('parser-api');
  assert.throws(() => validateVersionState(missingLock), /parser-api=missing/);
  assert.throws(
    () => validateVersionState(aligned(), { refType: 'tag', refName: 'v0.2.0' }),
    /release tag must be v0\.1\.0/,
  );
  assert.throws(
    () => validateVersionState(aligned(), { publishRequested: true, refName: 'development' }),
    /only from main/,
  );
});

test('dry-run release validation may run from development', () => {
  assert.equal(
    validateVersionState(aligned(), {
      requestedVersion: '0.1.0',
      refName: 'development',
      publishRequested: false,
    }),
    '0.1.0',
  );
});

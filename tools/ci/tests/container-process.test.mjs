import assert from 'node:assert/strict';
import test from 'node:test';
import { performance } from 'node:perf_hooks';
import { invokeContainer, spawnBounded } from '../check-container.mjs';

test('bounded process kills a child that ignores SIGTERM', () => {
  const started = performance.now();
  // Exit eventually even if SIGKILL regresses, so the test fails instead of hanging.
  const result = spawnBounded(process.execPath, ['-e', 'process.on("SIGTERM", () => {}); process.stdout.write("ready"); setTimeout(() => process.exit(99), 4000);'], { timeout: 1000 });
  assert.equal(result.stdout, 'ready');
  assert.equal(result.error?.code, 'ETIMEDOUT');
  assert.equal(result.signal, 'SIGKILL');
  assert(performance.now() - started < 5000, 'timeout must not await cooperative termination');
});

test('container invocation error removes only its named container', () => {
  const name = 'fuzzy-parser-ci-synthetic-test';
  const failed = { error: Object.assign(new Error('timeout'), { code: 'ETIMEDOUT' }) };
  const calls = [];
  const runner = (...args) => {
    calls.push(args);
    return calls.length === 1 ? failed : { status: 0 };
  };
  assert.equal(invokeContainer(['run', '--name', name, 'synthetic-image'], name, {}, runner), failed);
  assert.equal(calls.length, 2);
  assert.deepEqual(calls[1], ['docker', ['rm', '--force', name], { timeout: 10_000, maxBuffer: 65_536 }]);
});

test('cleanup failures are reported rather than claimed as removal', () => {
  for (const cleanup of [{ status: 1 }, { error: Object.assign(new Error('timeout'), { code: 'ETIMEDOUT' }) }]) {
    let calls = 0;
    assert.throws(() => invokeContainer(['run'], 'fuzzy-parser-ci-synthetic-test', {}, () => {
      calls += 1;
      return calls === 1 ? { error: new Error('invocation failed') } : cleanup;
    }), /removal of fuzzy-parser-ci-synthetic-test was not confirmed/);
    assert.equal(calls, 2);
  }
});

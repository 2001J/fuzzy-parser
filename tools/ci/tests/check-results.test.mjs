import assert from 'node:assert/strict';
import test from 'node:test';
import { assertJobResults, requiredJobs } from '../check-results.mjs';

const success = () => Object.fromEntries(requiredJobs.map((name) => [name, { result: 'success' }]));

test('aggregate gate accepts only complete successful results', () => {
  assert.doesNotThrow(() => assertJobResults(success()));
});

for (const state of ['failure', 'cancelled', 'skipped', undefined]) {
  test(`aggregate gate rejects ${String(state)} in every required job`, () => {
    for (const name of requiredJobs) {
      const results = success();
      results[name].result = state;
      assert.throws(() => assertJobResults(results));
    }
  });
}

test('aggregate gate rejects missing, malformed, or unexpected jobs', () => {
  for (const invalid of [null, {}, [], 'success']) assert.throws(() => assertJobResults(invalid));
  for (const name of requiredJobs) {
    const results = success();
    delete results[name];
    assert.throws(() => assertJobResults(results));
  }
  assert.throws(() => assertJobResults({ ...success(), unexpected: { result: 'success' } }));
});

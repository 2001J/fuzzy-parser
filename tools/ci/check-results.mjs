// Keep this list aligned with the gate job's explicit `needs` in ci.yml.
import assert from 'node:assert/strict';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

export const requiredJobs = ['quality', 'test', 'wasm', 'dependencies', 'container'];

export function assertJobResults(results) {
  assert(results && typeof results === 'object' && !Array.isArray(results), 'missing job results');
  assert.deepEqual(Object.keys(results).sort(), [...requiredJobs].sort(), 'required jobs changed or are missing');
  for (const name of requiredJobs) {
    assert.equal(results[name]?.result, 'success', `${name} did not succeed`);
  }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  assertJobResults(JSON.parse(process.env.CI_JOB_RESULTS ?? 'null'));
  console.log(`CI passed: ${requiredJobs.join(', ')}`);
}

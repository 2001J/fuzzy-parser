// Smoke tests for the real batch image, not a host adapter or deployment proof.
// Requires an already-built local linux/amd64 image; never pulls or publishes it.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

export function assertParseResult(response, inspected) {
  assert.equal(response.contract_version, '0.1');
  assert.equal(response.record_name, 'stock_check');
  assert.deepEqual(response.source_evidence.document, inspected);
  const records = response.content.mode === 'text'
    ? response.content.records
    : response.content.sheets.flatMap((sheet) => sheet.records);
  assert.equal(records.length, 2);
  for (const [name, value] of [['units', 42], ['available', true]]) {
    const field = records[0].parse.assignment.fields.find((field) => field.name === name);
    assert.equal(field?.candidates[0]?.normalized_value, value, `${name} must be assigned correctly`);
  }
  assert(records[1].parse.assignment.warnings.some((warning) => warning.code === 'required_field_missing'));
  assert.equal(records[1].parse.review.status, 'needs_review');
}

// SIGTERM can be caught/proxied, leaving spawnSync waiting forever. Kill the
// client unconditionally, then ask the daemon to remove only our own container.
export function spawnBounded(command, args, options = {}) {
  return spawnSync(command, args, {
    encoding: 'utf8', timeout: 30_000, maxBuffer: 4 * 1024 * 1024,
    ...options, killSignal: 'SIGKILL',
  });
}

export function invokeContainer(args, name, options = {}, runner = spawnBounded) {
  const result = runner('docker', args, options);
  if (result.error) {
    const cleanup = runner('docker', ['rm', '--force', name], { timeout: 10_000, maxBuffer: 65_536 });
    if (cleanup.error || cleanup.status !== 0) {
      throw new Error(`Docker invocation failed; removal of ${name} was not confirmed (${cleanup.error?.code ?? cleanup.status}). Inspect this named container before retrying.`, { cause: result.error });
    }
  }
  return result;
}

function main() {
  assert.equal(process.argv.length, 3, 'usage: node tools/ci/check-container.mjs <local-image>');
  const fixtures = resolve(dirname(fileURLToPath(import.meta.url)), '../../fixtures');
  const inspect = spawnBounded('docker', ['image', 'inspect', '--format', '{{json .}}', process.argv[2]], { maxBuffer: 1024 * 1024 });
  assert.ifError(inspect.error);
  assert.equal(inspect.status, 0, inspect.stderr);
  const metadata = JSON.parse(inspect.stdout);
  assert.equal(metadata.Os, 'linux');
  assert.equal(metadata.Architecture, 'amd64');
  const image = metadata.Id; // Use the inspected local image, not a moving tag.
  let checks = 0;

  function run(args, { input, code = 0, entrypoint, timeoutMs = 30_000, expectTimeout = false } = {}) {
    const name = `fuzzy-parser-ci-${randomUUID()}`;
    const options = ['run', '--rm', '--pull=never', '--platform', 'linux/amd64', '--name', name,
      '--network', 'none', '--read-only', '--cap-drop', 'ALL', '--security-opt', 'no-new-privileges',
      '--memory', '256m', '--pids-limit', '32', '--mount', `type=bind,source=${fixtures},target=/fixtures,readonly`];
    if (input !== undefined) options.push('-i');
    if (entrypoint) options.push('--entrypoint', entrypoint);
    const result = invokeContainer([...options, image, ...args], name, { input, timeout: timeoutMs });
    if (expectTimeout) {
      assert.equal(result.error?.code, 'ETIMEDOUT');
      assert.equal(result.signal, 'SIGKILL');
      const remaining = spawnBounded('docker', ['ps', '--all', '--quiet', '--filter', `name=^/${name}$`]);
      assert.ifError(remaining.error);
      assert.equal(remaining.status, 0, remaining.stderr);
      assert.equal(remaining.stdout.trim(), '', 'timed-out container must be gone');
      checks += 1;
      return result.stdout;
    }
    assert.ifError(result.error);
    assert.equal(result.signal, null);
    assert.equal(result.status, code, result.stderr);
    if (code === 0) assert.equal(result.stderr, '');
    else assert.equal(result.stdout, '');
    checks += 1;
    return code === 0 ? result.stdout : result.stderr;
  }

  assert.equal(run(['-u'], { entrypoint: '/usr/bin/id' }).trim(), '10001');
  assert.match(run(['--help']), /parse/);
  for (const path of ['/fixtures/runtime/review.txt', '/fixtures/runtime/review.csv', '/fixtures/xlsx/sample.xlsx']) {
    const inspected = JSON.parse(run(['inspect', path]));
    const response = JSON.parse(run(['parse', path, '--schema', '/fixtures/runtime/inventory-supported.json']));
    assertParseResult(response, inspected);
  }
  const input = '  Zoë 東京 😀 Count: 42 Enabled: true\nuntouched\n';
  const inspected = JSON.parse(run(['inspect', '--stdin'], { input }));
  const response = JSON.parse(run(['parse', '--stdin', '--schema', '/fixtures/runtime/inventory-supported.json'], { input }));
  assertParseResult(response, inspected);
  const invalid = JSON.parse(run(['inspect', '--stdin'], { input: Buffer.from([0xff]), code: 1 }));
  assert.deepEqual(invalid, {
    error: { error_contract_version: '0.1', code: 'invalid_utf8', valid_up_to: 0 },
    message: 'input is not valid UTF-8 at byte offset 0',
  });
  const missing = JSON.parse(run(['inspect', '/fixtures/does-not-exist.txt'], { code: 1 }));
  assert.deepEqual(missing, {
    error: { error_contract_version: '0.1', code: 'io_error', kind: 'not_found' },
    message: 'could not read input: file not found',
  });
  assert.match(run(['unknown-command'], { code: 2 }), /^usage: parser-cli /);
  // The finite sleep also bounds this regression if client termination breaks.
  assert.equal(run(['-c', 'trap "" TERM; printf ready; sleep 10'], {
    entrypoint: '/bin/sh', timeoutMs: 2000, expectTimeout: true,
  }), 'ready');
  console.log(JSON.stringify({ image, platform: 'linux/amd64', checks, nonRoot: true, readOnly: true, network: 'none', timeoutCleanup: true }));
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) main();

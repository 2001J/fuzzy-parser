// #11 evaluation/test tooling only, not an exported adapter or public API.
// No dependencies, host checkout, credentials, schema conversion, or network.
// Run with one explicit trusted CLI binary; see ADR 0006 for scope and gates.
import assert from 'node:assert/strict';
import { spawn, spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdtemp, readFile, rm, stat, writeFile } from 'node:fs/promises';
import { tmpdir, release } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { performance } from 'node:perf_hooks';

assert.equal(process.argv.length, 3, 'usage: node evaluate.mjs <trusted-parser-cli-path>');
const binary = resolve(process.argv[2]);
const fixtures = resolve(dirname(fileURLToPath(import.meta.url)), '../../fixtures');
const childEnv = { LANG: 'C.UTF-8' }; // Never forward the caller's environment.
const budgets = { inputBytes: 1048576, schemaBytes: 65536, stdoutBytes: 4194304, stderrBytes: 65536, timeoutMs: 5000 };
const report = { environment: { os: process.platform, architecture: process.arch, release: release(), node: process.version }, budgets, cases: [], failures: [], controls: [] };
const temporaryPaths = [];
const decoder = new TextDecoder('utf-8', { fatal: true });
let referenceChecks = 0;

async function prepared(kind, bytes, schema, work) {
  assert(['text', 'txt', 'csv', 'xlsx'].includes(kind));
  if (bytes.length > budgets.inputBytes) throw new Error('input_limit');
  if (Buffer.byteLength(schema) > budgets.schemaBytes) throw new Error('schema_limit');
  const directory = await mkdtemp(join(tmpdir(), 'parser-boundary-'));
  temporaryPaths.push(directory);
  try {
    assert.equal((await stat(directory)).mode & 0o777, 0o700);
    const schemaPath = join(directory, 'schema.json');
    const inputPath = join(directory, `input.${kind}`);
    await writeFile(schemaPath, schema, { mode: 0o600 });
    if (kind !== 'text') await writeFile(inputPath, bytes, { mode: 0o600 });
    const inputArgs = kind === 'text' ? ['--stdin'] : [inputPath];
    return await work({ directory, args: ['parse', ...inputArgs, '--schema', schemaPath], inputArgs, stdin: kind === 'text' ? bytes : Buffer.alloc(0) });
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
}

function direct(context, args = context.args) {
  const output = spawnSync(binary, args, { cwd: context.directory, env: childEnv, input: context.stdin, timeout: budgets.timeoutMs, maxBuffer: budgets.stdoutBytes, shell: false });
  assert.ifError(output.error);
  assert.equal(output.signal, null);
  return { code: output.status, stdout: output.stdout, stderr: output.stderr };
}

// Experimental byte/file bridge: preserve the CLI streams and wait for close
// before cleanup. Transport termination never masquerades as a parse result.
function invoke(context, options = {}) {
  const limits = { ...budgets, ...options };
  return new Promise((resolveResult) => {
    const start = performance.now();
    const child = spawn(options.binary ?? binary, context.args, { cwd: context.directory, env: childEnv, stdio: ['pipe', 'pipe', 'pipe'], shell: false });
    const chunks = { stdout: [], stderr: [] };
    const sizes = { stdout: 0, stderr: 0 };
    let failure = null;
    let escalation;
    function terminate(reason) {
      if (failure) return;
      failure = reason;
      child.kill('SIGTERM');
      escalation = setTimeout(() => child.kill('SIGKILL'), 100);
    }
    const timer = setTimeout(() => terminate('timeout'), limits.timeoutMs);
    const abort = () => terminate('cancelled');
    options.signal?.addEventListener('abort', abort, { once: true });
    if (options.signal?.aborted) abort();
    for (const name of ['stdout', 'stderr']) {
      child[name].on('data', (chunk) => {
        if (failure) return;
        sizes[name] += chunk.length;
        if (sizes[name] > limits[`${name}Bytes`]) terminate(`${name}_limit`);
        else chunks[name].push(chunk);
      });
    }
    child.on('error', () => { failure ??= 'spawn_error'; });
    child.stdin.on('error', (error) => { if (error.code !== 'EPIPE') terminate('stdin_error'); });
    child.on('spawn', () => {
      options.onSpawn?.();
      if (!options.holdStdin) child.stdin.end(context.stdin);
    });
    child.on('close', (code, signal) => {
      clearTimeout(timer);
      clearTimeout(escalation);
      options.signal?.removeEventListener('abort', abort);
      resolveResult({ code, signal, failure, elapsedMs: +(performance.now() - start).toFixed(3), pid: child.pid, stdout: failure ? Buffer.alloc(0) : Buffer.concat(chunks.stdout), stderr: failure ? Buffer.alloc(0) : Buffer.concat(chunks.stderr) });
    });
  });
}

function parses(response) {
  return response.content.mode === 'text' ? response.content.records.map((r) => r.parse) : response.content.sheets.flatMap((s) => s.records.map((r) => r.parse));
}

function checkSources(response) {
  const { document, blocks } = response.source_evidence;
  const values = document.blocks.map(({ value }) => Buffer.from(value.kind === 'Null' ? '' : String(value.value)));
  const covered = values.map((value) => new Uint8Array(value.length));
  for (const parse of parses(response)) {
    for (const candidate of [...parse.candidates, ...parse.assignment.fields.flatMap((f) => f.candidates), ...parse.assignment.unassigned_candidates]) {
      const reference = candidate.source_reference;
      assert(reference);
      const index = reference.block_index;
      const value = values[index];
      assert(value);
      const { byte_start: start, byte_end: end } = reference.span;
      assert(start >= 0 && start <= end && end <= value.length);
      assert.equal(decoder.decode(value.subarray(start, end)), candidate.raw_value);
      const rawKind = document.blocks[index].value.kind;
      assert.equal(reference.coordinate_space, ['Text', 'DateTimeText', 'Duration', 'Error'].includes(rawKind) ? 'raw_text_utf8' : 'rendered_value_utf8');
      assert(parse.candidates.some((detected) => JSON.stringify(detected.source_reference) === JSON.stringify(reference) && detected.candidate_type === candidate.candidate_type));
      covered[index].fill(1, start, end);
      referenceChecks += 1;
    }
  }
  assert.equal(blocks.length, document.blocks.length);
  for (const [index, coverage] of blocks.entries()) {
    assert.equal(coverage.block_index, index);
    if (coverage.role !== 'parsed') { assert(['header', 'excluded'].includes(coverage.role)); assert(coverage.reason); continue; }
    for (const { byte_start: start, byte_end: end } of coverage.unused_spans) {
      assert(start >= 0 && start <= end && end <= values[index].length);
      decoder.decode(values[index].subarray(start, end));
      assert(covered[index].subarray(start, end).every((byte) => byte === 0));
      covered[index].fill(1, start, end);
    }
    assert(covered[index].every((byte) => byte === 1), 'every canonical source byte is accounted for');
  }
}

async function success(name, kind, bytes, schema, expectedRecords = 2) {
  return prepared(kind, bytes, JSON.stringify(schema), async (context) => {
    const baseline = direct(context);
    const result = await invoke(context);
    const repeated = await invoke(context);
    assert.equal(result.failure, null);
    assert.equal(result.code, 0);
    assert.equal(result.stderr.length, 0);
    assert.equal(repeated.failure, null);
    assert.equal(repeated.code, 0);
    assert.deepEqual(result.stdout, baseline.stdout);
    assert.deepEqual(result.stderr, baseline.stderr);
    assert.deepEqual(repeated.stdout, result.stdout);
    assert.deepEqual(repeated.stderr, result.stderr);
    const response = JSON.parse(result.stdout);
    assert.equal(response.contract_version, '0.1');
    assert.equal(response.parser_version, '0.1.0');
    assert.equal(response.record_name, schema.record_name);
    const records = parses(response);
    assert.equal(records.length, expectedRecords);
    for (const [index, expected] of [42, true].entries()) {
      const field = records[0].assignment.fields.find((f) => f.name === schema.fields[index].name);
      assert(field);
      assert.equal(field.candidates[0].normalized_value, expected);
    }
    const inspected = direct(context, ['inspect', ...context.inputArgs]);
    assert.equal(inspected.code, 0);
    assert.equal(inspected.stderr.length, 0);
    assert.deepEqual(response.source_evidence.document, JSON.parse(inspected.stdout));
    checkSources(response);
    if (!name.startsWith('scale-')) {
      assert(records[1].assignment.warnings.some((w) => w.code === 'required_field_missing'));
      assert.equal(records[1].review.status, 'needs_review');
      if (kind === 'xlsx') {
        const raw = response.source_evidence.document.blocks.map((b) => b.value.kind);
        for (const type of ['Decimal', 'Boolean', 'DateTime', 'Null']) assert(raw.includes(type));
      }
    }
    report.cases.push({ name, inputBytes: bytes.length, outputBytes: result.stdout.length, records: records.length, milliseconds: [result.elapsedMs, repeated.elapsedMs] });
  });
}

async function parserFailure(name, kind, bytes, schema, expectedCode) {
  await prepared(kind, bytes, schema, async (context) => {
    const baseline = direct(context);
    const result = await invoke(context);
    assert.equal(result.failure, null);
    assert.equal(result.code, 1);
    assert.equal(result.stdout.length, 0);
    assert.deepEqual(result.stdout, baseline.stdout);
    assert.deepEqual(result.stderr, baseline.stderr);
    assert.equal(JSON.parse(result.stderr).error.code, expectedCode);
    report.failures.push({ name, code: expectedCode, exactStderrParity: true });
  });
}

const profiles = [];
for (const name of ['attendance-supported', 'inventory-supported']) {
  const schema = JSON.parse(await readFile(join(fixtures, 'runtime', `${name}.json`)));
  profiles.push(schema);
  for (const [kind, fixture] of [['text', 'runtime/review.txt'], ['txt', 'runtime/review.txt'], ['csv', 'runtime/review.csv'], ['xlsx', 'xlsx/sample.xlsx']]) {
    await success(`${name}/${kind}`, kind, await readFile(join(fixtures, fixture)), schema);
  }
}
const schema = profiles[1];
const schemaJson = JSON.stringify(schema);
await parserFailure('invalid-schema-json', 'text', Buffer.from('42 true'), '{', 'schema_parse_error');
await parserFailure('unsupported-schema-version', 'text', Buffer.from('42 true'), JSON.stringify({ ...schema, schema_version: '999' }), 'schema_validation_error');
for (const type of ['text', 'person_name', 'datetime']) {
  const unsupported = structuredClone(schema);
  unsupported.fields[0].field_type = type;
  await parserFailure(`unsupported-${type}`, 'text', Buffer.from('42 true'), JSON.stringify(unsupported), 'schema_field_type_unsupported');
}
await parserFailure('invalid-utf8', 'txt', Buffer.from([0xff]), schemaJson, 'invalid_utf8');
await parserFailure('invalid-csv', 'csv', Buffer.from('Name,Count\n"unclosed,42'), schemaJson, 'invalid_csv');
await parserFailure('invalid-xlsx', 'xlsx', Buffer.from('not a workbook'), schemaJson, 'invalid_xlsx');
await parserFailure('text-line-limit', 'text', Buffer.alloc(65537, 120), schemaJson, 'line_too_long');

for (const count of [10, 100, 500]) {
  const csv = Buffer.from('Name,Count,Enabled\n' + 'sample,42,true\n'.repeat(count));
  await success(`scale-${count}`, 'csv', csv, schema, count);
}
for (const [name, bytes, oversizedSchema] of [
  ['input_limit', Buffer.alloc(budgets.inputBytes + 1), schemaJson],
  ['schema_limit', Buffer.alloc(0), ' '.repeat(budgets.schemaBytes + 1)],
]) {
  await assert.rejects(prepared('text', bytes, oversizedSchema, () => assert.fail('must reject before invocation')), { message: name });
  report.controls.push(name);
}
await prepared('text', Buffer.from('42 true'), schemaJson, async (context) => {
  const controller = new AbortController();
  const controls = [
    ['cancelled', { holdStdin: true, signal: controller.signal, onSpawn: () => setTimeout(() => controller.abort(), 25) }],
    ['timeout', { holdStdin: true, timeoutMs: 25 }],
    ['stdout_limit', { stdoutBytes: 128 }],
    ['spawn_error', { binary: join(context.directory, 'missing-binary') }],
  ];
  for (const [name, options] of controls) {
    const result = await invoke(context, options);
    assert.equal(result.failure, name);
    assert.equal(result.stdout.length, 0);
    assert.equal(result.stderr.length, 0);
    if (result.pid) assert.throws(() => process.kill(result.pid, 0), { code: 'ESRCH' });
    assert(result.elapsedMs < budgets.timeoutMs);
    report.controls.push({ name, milliseconds: result.elapsedMs, childReaped: true });
  }
});
await prepared('text', Buffer.from('42 true'), '{', async (context) => {
  const result = await invoke(context, { stderrBytes: 8 });
  assert.equal(result.failure, 'stderr_limit');
  assert.equal(result.stderr.length, 0);
  report.controls.push('stderr_limit');
});
for (const path of temporaryPaths) await assert.rejects(stat(path), { code: 'ENOENT' });
const artifact = await readFile(binary);
report.artifact = { bytes: artifact.length, sha256: createHash('sha256').update(artifact).digest('hex') };
report.sourceReferenceChecks = referenceChecks;
report.cleanedTemporaryDirectories = temporaryPaths.length;
report.parentNodeMaxRssKiB = process.resourceUsage().maxRSS;
try { report.containerMemoryPeakBytes = Number((await readFile('/sys/fs/cgroup/memory.peak', 'utf8')).trim()); } catch { report.containerMemoryPeakBytes = null; }
console.log(JSON.stringify(report, null, 2));

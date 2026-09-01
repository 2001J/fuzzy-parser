import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { cp, mkdtemp, readFile, rm, stat, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { basename, dirname, join, resolve } from 'node:path';
import { performance } from 'node:perf_hooks';
import { tmpdir } from 'node:os';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { Worker } from 'node:worker_threads';

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, '../..');
const pkg = join(here, 'pkg', 'fuzzy_parser_wasm_evaluation.js');
const wasm = join(here, 'pkg', 'fuzzy_parser_wasm_evaluation_bg.wasm');
const oracle = join(here, 'target', 'debug', 'native-byte-oracle');
const limits = { inputBytes: 1024 * 1024, schemaBytes: 64 * 1024, outputBytes: 4 * 1024 * 1024 };
const report = { environment: { node: process.version, platform: process.platform, arch: process.arch }, limits, successes: [], errors: [], probes: {}, artifacts: {} };

const require = createRequire(import.meta.url);
const cjs = require(pkg);
const esm = await import(pathToFileURL(pkg).href);
const cjsCall = cjs.parse_bytes_json;
const esmCall = esm.parse_bytes_json ?? esm.default?.parse_bytes_json;
assert.equal(typeof cjsCall, 'function', 'CJS loader must expose parse_bytes_json');
assert.equal(typeof esmCall, 'function', 'ESM loader must expose parse_bytes_json');

function bounded(call, format, bytes, filename, schema) {
  if (bytes.length > limits.inputBytes) throw new Error('input_limit');
  if (Buffer.byteLength(schema) > limits.schemaBytes) throw new Error('schema_limit');
  const response = call(format, bytes, filename, schema);
  if (Buffer.byteLength(response) > limits.outputBytes) throw new Error('output_limit');
  return response;
}

function native(request) {
  return new Promise((resolvePromise, reject) => {
    const child = spawn(oracle, [], { stdio: ['pipe', 'pipe', 'pipe'] });
    let stdout = '', stderr = '';
    child.stdout.setEncoding('utf8').on('data', (chunk) => { stdout += chunk; });
    child.stderr.setEncoding('utf8').on('data', (chunk) => { stderr += chunk; });
    child.on('error', reject).on('close', (code) => {
      if (code !== 0) reject(new Error(`oracle exited ${code}: ${stderr}`));
      else resolvePromise(JSON.parse(stdout).json);
    });
    child.stdin.end(JSON.stringify({ ...request, bytes: [...request.bytes] }));
  });
}

function nodeProbe(args) {
  return new Promise((resolvePromise, reject) => {
    const child = spawn(process.execPath, args, { stdio: ['ignore', 'pipe', 'pipe'] });
    let stdout = '', stderr = '';
    child.stdout.setEncoding('utf8').on('data', (chunk) => { stdout += chunk; });
    child.stderr.setEncoding('utf8').on('data', (chunk) => { stderr += chunk; });
    child.on('error', reject).on('close', (code) => resolvePromise({ code, stdout, stderr }));
  });
}

async function loadProbe(mode) {
  const expression = mode === 'cjs'
    ? "const {performance}=require('node:perf_hooks');const t=performance.now();require(process.argv[1]);console.log(performance.now()-t)"
    : "const {performance}=await import('node:perf_hooks');const t=performance.now();await import(process.argv[1]);console.log(performance.now()-t)";
  const target = mode === 'cjs' ? pkg : pathToFileURL(pkg).href;
  const result = await nodeProbe(['-e', expression, target]);
  assert.equal(result.code, 0, `${mode}: cold load exits successfully`);
  return Number(result.stdout.trim());
}

async function assetFailureProbe(kind) {
  const directory = await mkdtemp(join(tmpdir(), 'fuzzy-parser-wasm-'));
  const copyPkg = join(directory, basename(pkg));
  const copyWasm = join(directory, basename(wasm));
  try {
    await cp(pkg, copyPkg);
    if (kind === 'corrupt') await writeFile(copyWasm, Buffer.from('not wasm'));
    const result = await nodeProbe(['-e', "const m=require(process.argv[1]);m.parse_bytes_json('text',Buffer.from('x'),null,process.argv[2])", copyPkg, inventory]);
    assert.notEqual(result.code, 0, `${kind} asset fails before parsing`);
    return result.code;
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
}

function assertSourceReferences(response) {
  const document = response.source_evidence.document;
  for (const record of response.content.records ?? response.content.sheets.flatMap((sheet) => sheet.records)) {
    const assignment = record.parse.assignment;
    const candidates = [
      ...record.parse.candidates,
      ...assignment.fields.flatMap((field) => field.candidates),
      ...assignment.unassigned_candidates,
    ];
    for (const candidate of candidates) {
      if (!candidate.source_reference) continue;
      const reference = candidate.source_reference;
      const block = document.blocks[reference.block_index];
      assert(block, 'source reference resolves block');
      const rawKinds = ['Text', 'DateTimeText', 'Duration', 'Error'];
      assert(reference.coordinate_space === (rawKinds.includes(block.value.kind) ? 'raw_text_utf8' : 'rendered_value_utf8'), 'coordinate space matches RawValue kind');
      const blockText = block.value.kind === 'Null' ? '' : (typeof block.value.value === 'string' ? block.value.value : String(block.value.value));
      const blockBytes = Buffer.from(blockText, 'utf8');
      const { byte_start: start, byte_end: end } = reference.span;
      const utf8Boundary = (offset) => Buffer.from(blockBytes.subarray(0, offset).toString('utf8')).equals(blockBytes.subarray(0, offset));
      assert(
        Number.isInteger(start) && Number.isInteger(end) && start >= 0 && end >= start && end <= blockBytes.length && utf8Boundary(start) && utf8Boundary(end),
        'source reference byte span is ordered',
      );
      assert.equal(Buffer.from(blockBytes.subarray(start, end)).toString('utf8'), candidate.raw_value, 'source span exactly resolves candidate raw value');
    }
  }
}

async function parity(name, format, bytes, schema, filename = null) {
  const request = { format, bytes, schema, filename };
  const nativeResult = await native(request);
  let parsed;
  for (const [loader, call] of [['cjs', cjsCall], ['esm', esmCall]]) {
    const before = process.memoryUsage().heapUsed;
    const started = performance.now();
    const result = bounded(call, format, bytes, filename, schema);
    const elapsedMs = performance.now() - started;
    assert.equal(result, nativeResult, `${name}/${loader}: exact native byte-path parity`);
    parsed = JSON.parse(result);
    if (!parsed.error) assertSourceReferences(parsed);
    assert.equal(bounded(call, format, bytes, filename, schema), result, `${name}/${loader}: repeatable`);
    report.successes.push({ name, loader, format, inputBytes: bytes.length, outputBytes: Buffer.byteLength(result), elapsedMs, heapDeltaBytes: process.memoryUsage().heapUsed - before });
  }
  return { result: nativeResult, parsed };
}

const fixture = (...parts) => readFile(join(root, 'fixtures', ...parts));
const profiles = await Promise.all(['attendance-supported', 'inventory-supported'].map(async (name) => [name, await readFile(join(root, 'fixtures/runtime', `${name}.json`), 'utf8')]));
for (const [profile, schema] of profiles) {
  for (const [format, path] of [['text', ['runtime', 'review.txt']], ['txt', ['runtime', 'review.txt']], ['csv', ['runtime', 'review.csv']], ['xlsx', ['xlsx', 'sample.xlsx']]]) {
    const bytes = await fixture(...path);
    const { parsed } = await parity(`${profile}/${format}`, format, bytes, schema, format === 'text' ? null : basename(path.at(-1)));
    assert.equal(parsed.record_name, JSON.parse(schema).record_name);
  }
}

const inventory = profiles.find(([name]) => name === 'inventory-supported')[1];
for (const [format, path] of [['txt', ['runtime', 'review.txt']], ['csv', ['runtime', 'review.csv']], ['xlsx', ['xlsx', 'sample.xlsx']]]) {
  const { parsed } = await parity(`absent-filename/${format}`, format, await fixture(...path), inventory);
  assert.equal(parsed.source_evidence.document.source.file_name, null, `${format}: absent filename remains absent`);
}
const typedXlsx = await parity('typed-xlsx', 'xlsx', await fixture('xlsx', 'sample.xlsx'), inventory, 'sample.xlsx');
assert.deepEqual(
  typedXlsx.parsed.source_evidence.document.blocks.slice(5, 8).map((block) => block.value.kind),
  ['Decimal', 'Boolean', 'DateTime'],
  'XLSX preserves typed cell values through the byte boundary',
);
const unicodeBytes = Buffer.from((await fixture('text', 'unicode-whitespace.txt.hex')).toString('utf8').replaceAll(/\s+/g, ''), 'hex');
const unicode = await parity('unicode-whitespace', 'txt', unicodeBytes, inventory, 'unicode-whitespace.txt');
assert.equal(unicode.parsed.source_evidence.document.source.size_bytes, unicodeBytes.length);
assert.equal(unicode.parsed.source_evidence.document.blocks[0].value.value, '  Zoë—東京 😀\t ');
assert.equal(unicode.parsed.source_evidence.document.blocks[2].value.value, 'Café');
const unicodeXlsxBytes = Buffer.from((await fixture('xlsx', 'unicode.xlsx.hex')).toString('utf8').replaceAll(/\s+/g, ''), 'hex');
const unicodeXlsx = await parity('unicode-xlsx', 'xlsx', unicodeXlsxBytes, inventory, 'résumé 東京 😀.xlsx');
assert.equal(unicodeXlsx.parsed.source_evidence.document.source.file_name, 'résumé 東京 😀.xlsx');
assert.equal(unicodeXlsx.parsed.source_evidence.document.blocks[4].value.value, '  Zoë 東京 😀  ');
for (const count of [10, 100, 500]) await parity(`scale-${count}`, 'csv', Buffer.from(`Name,Count,Enabled\n${'sample,42,true\n'.repeat(count)}`), inventory, 'scale.csv');
for (const fieldType of ['text', 'person_name']) {
  const { parsed } = await parity(
    `supported-${fieldType}`,
    'text',
    Buffer.from('Ada Lovelace'),
    JSON.stringify({ ...JSON.parse(inventory), fields: [{ ...JSON.parse(inventory).fields[0], field_type: fieldType }] }),
  );
  assert.equal(parsed.content.records[0].parse.review.status, 'needs_review');
}
for (const [name, format, bytes, schema, code] of [
  ['invalid-schema', 'text', Buffer.from('42'), '{', 'schema_parse_error'],
  ['bad-version', 'text', Buffer.from('42'), JSON.stringify({ ...JSON.parse(inventory), schema_version: '999' }), 'schema_validation_error'],
  ['unsupported-datetime', 'text', Buffer.from('42'), JSON.stringify({ ...JSON.parse(inventory), fields: [{ ...JSON.parse(inventory).fields[0], field_type: 'datetime' }] }), 'schema_field_type_unsupported'],
  ['invalid-enum', 'text', Buffer.from('42'), JSON.stringify({ ...JSON.parse(inventory), fields: [{ ...JSON.parse(inventory).fields[0], field_type: { enum: { values: [{ value: 'two words', aliases: [] }] } } }] }), 'schema_enum_definition_unsupported'],
  ['invalid-utf8', 'txt', Buffer.from([0xff]), inventory, 'invalid_utf8'],
  ['invalid-csv', 'csv', Buffer.from('name,count\n"open,42'), inventory, 'invalid_csv'],
  ['invalid-xlsx', 'xlsx', Buffer.from('not workbook'), inventory, 'invalid_xlsx'],
  ['line-limit', 'text', Buffer.alloc(65_537, 120), inventory, 'line_too_long'],
  ['unsupported-format', 'unknown', Buffer.from('42'), inventory, 'unsupported_input'],
]) {
  const { result } = await parity(name, format, bytes, schema, format + '.bin');
  assert.equal(JSON.parse(result).error?.code, code, `${name}: safe error report`);
  report.errors.push({ name, code });
}

const large = await parity('large-json-integer', 'csv', Buffer.from('count\n9007199254740993\n'), JSON.stringify({ schema_version: '0.1', record_name: 'integer', fields: [{ name: 'count', field_type: 'integer', required: true, multiple: false, aliases: [], constraints: [] }], options: { allow_unknown_fields: true } }), 'integer.csv');
assert(large.result.includes('9007199254740993'), 'wire JSON preserves the integer literal');
assert.equal(
  JSON.parse(large.result).source_evidence.document.blocks.find((block) => block.value.value === '9007199254740993').value.value,
  '9007199254740993',
  'source evidence retains the >2^53 literal as a JSON string',
);
report.probes.largeInteger = 'wire/source string exact; no JavaScript numeric conversion is required for provenance';
for (const [name, bytes, schema] of [['input', Buffer.alloc(limits.inputBytes + 1), inventory], ['schema', Buffer.alloc(0), ' '.repeat(limits.schemaBytes + 1)]]) assert.throws(() => bounded(cjsCall, 'text', bytes, null, schema), new Error(`${name}_limit`));

const entrySignal = new SharedArrayBuffer(4);
const entryView = new Int32Array(entrySignal);
const worker = new Worker(join(here, 'worker.cjs'), { workerData: { pkg, schema: inventory, signal: entrySignal } });
const cancellation = await new Promise((resolvePromise, reject) => {
  let complete = false;
  worker.on('message', async (message) => {
    if (message === 'complete') complete = true;
    if (message !== 'ready') return;
    const deadline = performance.now() + 1000;
    while (Atomics.load(entryView, 0) !== 1 && performance.now() < deadline) await new Promise((resolvePromise) => setTimeout(resolvePromise, 1));
    assert.equal(Atomics.load(entryView, 0), 1, 'Rust boundary signals parser entry');
    await new Promise((resolvePromise) => setTimeout(resolvePromise, 20));
    assert.equal(complete, false, 'no parser completion arrived before termination');
    const started = performance.now();
    await worker.terminate();
    resolvePromise({ elapsedMs: performance.now() - started, entryObserved: true, completionObserved: complete });
  });
  worker.once('error', reject);
});
report.probes.workerCancellation = cancellation;
report.probes.copyEvidence = {
  generatedGlueCopiesInput: (await readFile(pkg, 'utf8')).includes('getUint8ArrayMemory0().set(arg, ptr / 1)'),
  processArrayBufferBytes: process.memoryUsage().arrayBuffers,
};
report.probes.coldLoadMs = { cjs: await loadProbe('cjs'), esm: await loadProbe('esm') };
report.probes.assetFailures = { missing: await assetFailureProbe('missing'), corrupt: await assetFailureProbe('corrupt') };
for (const path of [pkg, wasm, oracle]) { const bytes = await readFile(path); report.artifacts[basename(path)] = { bytes: bytes.length, sha256: createHash('sha256').update(bytes).digest('hex') }; }
for (const path of [pkg, wasm]) await stat(path);
console.log(JSON.stringify(report, null, 2));

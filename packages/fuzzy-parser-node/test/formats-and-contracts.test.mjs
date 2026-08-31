import assert from 'node:assert/strict';
import test from 'node:test';
import {
  assertSourceReferences,
  fixture,
  loadCjs,
  loadEsm,
  publicRequest,
  schemas,
} from './helpers.mjs';

const loaders = [
  ['cjs', loadCjs()],
  ['esm', await loadEsm()],
];

test('CJS and ESM invoke text, TXT, CSV, and XLSX for two unrelated profiles', async () => {
  const inputs = [
    ['text', await fixture('runtime', 'review.txt'), undefined],
    ['txt', await fixture('runtime', 'review.txt'), 'review.txt'],
    ['csv', await fixture('runtime', 'review.csv'), 'review.csv'],
    ['xlsx', await fixture('xlsx', 'sample.xlsx'), 'sample.xlsx'],
  ];
  for (const [loader, api] of loaders) {
    for (const [profile, schema] of Object.entries(schemas)) {
      for (const [format, bytes, filename] of inputs) {
        const response = await api.parse(publicRequest(format, bytes, schema, filename));
        assert.equal(response.record_name, schema.record_name, `${loader}/${profile}/${format}`);
        assert.equal(response.contract_version, '0.1');
        assert.equal(response.parser_version, '0.1.0');
        assertSourceReferences(response);
      }
    }
  }
});

test('Unicode whitespace, safe filename metadata, typed and blank XLSX values remain raw', async () => {
  const { parse } = await loadEsm();
  const hex = (await fixture('text', 'unicode-whitespace.txt.hex')).toString('utf8').replaceAll(/\s+/g, '');
  const unicode = Buffer.from(hex, 'hex');
  const txt = await parse(publicRequest('txt', unicode, schemas['inventory-supported'], 'résumé 東京 😀.txt'));
  assert.equal(txt.source_evidence.document.source.file_name, 'résumé 東京 😀.txt');
  assert.equal(txt.source_evidence.document.source.size_bytes, unicode.length);
  assert.equal(txt.source_evidence.document.blocks[0].value.value, '  Zoë—東京 😀\t ');
  assert.equal(txt.source_evidence.document.blocks[2].value.value, 'Café');

  const xlsx = await parse(publicRequest(
    'xlsx',
    await fixture('xlsx', 'sample.xlsx'),
    schemas['inventory-supported'],
    'sample.xlsx',
  ));
  assert.deepEqual(
    xlsx.source_evidence.document.blocks.slice(5, 8).map((block) => block.value.kind),
    ['Decimal', 'Boolean', 'DateTime'],
  );
  assert.deepEqual(
    xlsx.source_evidence.document.blocks.slice(10, 12).map((block) => block.value.kind),
    ['Null', 'Null'],
  );
  assertSourceReferences(xlsx);
});

test('explicit table options preserve selected-row provenance and unused content', async () => {
  const { parse } = loadCjs();
  const request = publicRequest('csv', Buffer.from('meta,value\nName,Count\nAda,42\nBob,7\n'), schemas['inventory-supported'], 'rows.csv');
  request.options = {
    tableSelection: {
      header: { mode: 'row', row: 2 },
      includeRows: [{ start: 3, end: 3 }],
      sheets: { mode: 'all' },
    },
  };
  const response = await parse(request);
  assert.equal(response.content.sheets[0].records.length, 1);
  const evidence = response.source_evidence.table.sheets[0].rows;
  assert.equal(evidence.find((row) => row.source_row === 2).role, 'header');
  assert.equal(evidence.find((row) => row.source_row === 3).role, 'parsed');
  assert.equal(evidence.find((row) => row.source_row === 4).role, 'excluded');
  assert(response.source_evidence.blocks.some((block) => block.unused_spans.length > 0));
});

test('safe parser failures stay structured and distinct from adapter failures', async () => {
  const { parse, ParserFailure, AdapterError } = await loadEsm();
  await assert.rejects(
    parse(publicRequest('text', Buffer.from('private input'), '{')),
    (error) => {
      assert(error instanceof ParserFailure);
      assert.equal(error.code, 'schema_parse_error');
      assert.equal(error.report.error.error_contract_version, '0.1');
      assert(!JSON.stringify(error.report).includes('private input'));
      return true;
    },
  );
  await assert.rejects(
    parse(publicRequest('txt', Buffer.from([0xff]), schemas['inventory-supported'], 'bad.txt')),
    (error) => error instanceof ParserFailure && error.code === 'invalid_utf8',
  );
  await assert.rejects(
    parse({ input: { format: 'text', bytes: Buffer.from('x'), filename: '../private.txt' }, schema: schemas['inventory-supported'] }),
    (error) => error instanceof AdapterError && error.code === 'INVALID_REQUEST',
  );
});

test('#17 schema exact/one-over and response limits cross the package unchanged', async () => {
  const { parse, ParserFailure } = loadCjs();
  const base = JSON.stringify(schemas['inventory-supported']);
  const exactSchema = `${base}${' '.repeat(1024 * 1024 - Buffer.byteLength(base))}`;
  const exact = await parse(publicRequest('text', Buffer.from('42'), exactSchema));
  assert.equal(exact.contract_version, '0.1');

  await assert.rejects(
    parse(publicRequest('text', Buffer.from('42'), `${exactSchema} `)),
    (error) => {
      assert(error instanceof ParserFailure);
      assert.deepEqual(
        { code: error.code, resource: error.report.error.resource, limit: error.report.error.limit, actual: error.report.error.actual },
        { code: 'resource_limit', resource: 'schema_bytes', limit: 1024 * 1024, actual: 1024 * 1024 + 1 },
      );
      return true;
    },
  );

  const rows = `value\n${'synthetic evidence row\n'.repeat(50_000)}`;
  await assert.rejects(
    parse(publicRequest('csv', Buffer.from(rows), schemas['inventory-supported'], 'large.csv'), { timeoutMs: 120_000 }),
    (error) => {
      assert(error instanceof ParserFailure);
      assert.equal(error.code, 'resource_limit');
      assert.equal(error.report.error.resource, 'response_bytes');
      assert(error.report.error.actual > error.report.error.limit);
      return true;
    },
  );
});

test('package message limits reject oversize values before Worker allocation', async () => {
  const { parse, AdapterError, PACKAGE_LIMITS } = loadCjs();
  const options = { padding: 'x'.repeat(PACKAGE_LIMITS.maxOptionsBytes) };
  await assert.rejects(
    parse({ ...publicRequest('text', Buffer.from('x')), options }),
    (error) => error instanceof AdapterError && error.code === 'MESSAGE_LIMIT',
  );
});

test('repeated calls are byte-deterministic and do not log input, schema, environment, or diagnostics', async () => {
  const api = loadCjs();
  const marker = 'RAW_PRIVATE_MARKER_97';
  const environmentMarker = 'ENV_PRIVATE_MARKER_31';
  process.env.FUZZY_PARSER_PRIVATE_TEST_MARKER = environmentMarker;
  const logs = [];
  const methods = ['log', 'info', 'warn', 'error', 'debug'];
  const originals = Object.fromEntries(methods.map((method) => [method, console[method]]));
  for (const method of methods) console[method] = (...values) => logs.push([method, values]);
  try {
    const request = publicRequest('text', Buffer.from(`${marker} 42`));
    const first = await api.parse(request);
    const second = await api.parse(request);
    assert.equal(JSON.stringify(first), JSON.stringify(second));
  } finally {
    for (const method of methods) console[method] = originals[method];
    delete process.env.FUZZY_PARSER_PRIVATE_TEST_MARKER;
  }
  assert.deepEqual(logs, []);
});

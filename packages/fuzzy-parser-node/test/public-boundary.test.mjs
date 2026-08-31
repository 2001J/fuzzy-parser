import assert from 'node:assert/strict';
import test from 'node:test';
import { loadCjs, loadEsm, packageRoot, publicRequest, schemas } from './helpers.mjs';

test('CJS and ESM expose the same documented public surface', async () => {
  const cjs = loadCjs();
  const esm = await loadEsm();
  for (const loaded of [cjs, esm]) {
    assert.equal(typeof loaded.parse, 'function');
    assert.equal(typeof loaded.AdapterError, 'function');
    assert.equal(typeof loaded.ParserFailure, 'function');
  }
});

test('raw text parses through the public Worker boundary with exact evidence', async () => {
  const { parse } = await loadEsm();
  const bytes = Buffer.from('Widget 42 true\n', 'utf8');
  const response = await parse(publicRequest('text', bytes));
  assert.equal(response.contract_version, '0.1');
  assert.equal(response.parser_version, '0.1.0');
  assert.equal(response.record_name, schemas['inventory-supported'].record_name);
  assert.equal(response.source_evidence.document.blocks[0].value.value, 'Widget 42 true');
  const candidate = response.content.records[0].parse.candidates.find(
    (entry) => entry.raw_value === '42',
  );
  assert(candidate?.source_reference, 'candidate retains source reference');
  assert.deepEqual(candidate.source_reference.span, candidate.source_span);
});

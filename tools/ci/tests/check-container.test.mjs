import assert from 'node:assert/strict';
import test from 'node:test';
import { assertParseResult } from '../check-container.mjs';

const inspected = { id: 'synthetic-document', blocks: [] };
function success() {
  return {
    contract_version: '0.1', record_name: 'stock_check', source_evidence: { document: inspected },
    content: { mode: 'text', records: [
      { parse: { assignment: { fields: [
        { name: 'units', candidates: [{ normalized_value: 42 }] },
        { name: 'available', candidates: [{ normalized_value: true }] },
      ] } } },
      { parse: { assignment: { warnings: [{ code: 'required_field_missing' }] }, review: { status: 'needs_review' } } },
    ] },
  };
}

test('container contract guard accepts correct text and table results', () => {
  const response = success();
  assert.doesNotThrow(() => assertParseResult(response, inspected));
  response.content = { mode: 'table', sheets: [{ records: response.content.records }] };
  assert.doesNotThrow(() => assertParseResult(response, inspected));
});

test('container contract guard rejects successful exit with broken semantics', () => {
  const mutations = [
    (r) => { r.contract_version = 'wrong'; },
    (r) => { r.record_name = 'wrong'; },
    (r) => { r.source_evidence = {}; },
    (r) => { r.source_evidence.document = { blocks: ['lost source'] }; },
    (r) => { r.content.records[0].parse.assignment.fields[0].candidates[0].normalized_value = 2; },
    (r) => { r.content.records[0].parse.assignment.fields[1].candidates[0].normalized_value = false; },
    (r) => { r.content.records[1].parse.assignment.warnings = []; },
    (r) => { r.content.records[1].parse.review.status = 'approved'; },
    (r) => { r.content.records.pop(); },
  ];
  for (const mutate of mutations) {
    const response = success();
    mutate(response);
    assert.throws(() => assertParseResult(response, inspected));
  }
});

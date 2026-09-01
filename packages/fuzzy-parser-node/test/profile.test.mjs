import assert from 'node:assert/strict';
import test from 'node:test';
import { fixture, loadCjs, loadEsm } from './helpers.mjs';

const profileDefinition = {
  name: 'generic-contact',
  version: '2026-08',
  recordName: 'contact',
  fields: [
    { name: 'person', fieldType: 'person_name', required: true, aliases: ['Name'] },
    { name: 'phone', fieldType: 'phone_number', required: true },
    { name: 'amount', fieldType: 'currency' },
    { name: 'notes', fieldType: 'text' },
  ],
};

test('a reusable profile validates once and preserves review and source evidence across text CSV and XLSX', async () => {
  for (const load of [loadCjs, loadEsm]) {
    const api = await load();
    const profile = await api.defineProfile(profileDefinition);
    assert.equal(profile.name, 'generic-contact');
    assert.equal(profile.version, '2026-08');
    assert(Object.isFrozen(profile.schema.fields));
    const text = await api.parseProfile(profile, {
      format: 'text',
      bytes: new TextEncoder().encode('Name: Ada; phone: +255700000000; Amount: $42; Notes: early'),
    });
    const csv = await api.parseProfile(profile, {
      format: 'csv', bytes: await fixture('conformance', 'shared.csv'), filename: 'contacts.csv',
    });
    const xlsx = await api.parseProfile(profile, {
      format: 'xlsx', bytes: await fixture('xlsx', 'sample.xlsx'), filename: 'contacts.xlsx',
    });
    assert(api.records(text).some((record) => record.parse.assignment.fields.some((field) => field.name === 'amount')));
    assert(api.records(csv).length > 0);
    assert(api.records(xlsx).length > 0);
    assert.equal(api.reviewRecords(text).length, api.records(text).filter((record) => record.parse.review.status === 'needs_review').length);
    const unresolved = api.unresolvedEvidence(csv);
    assert.equal(unresolved.source, csv.source_evidence);
    assert(unresolved.records.every((entry) => entry.candidates.length > 0));
  }
});

test('profile capability validation happens before application input is supplied', async () => {
  const api = await loadCjs();
  await assert.rejects(
    api.defineProfile({ name: 'invalid', version: '1', fields: [{ name: 'when', fieldType: 'datetime' }] }),
    (error) => error.name === 'ParserFailure' && error.code === 'schema_field_type_unsupported',
  );
  assert.throws(
    () => api.parseProfile({ schema: {} }, { format: 'text', bytes: new Uint8Array() }),
    (error) => error.name === 'AdapterError' && error.code === 'INVALID_REQUEST',
  );
  await assert.rejects(
    api.defineProfile({ ...profileDefinition, unexpected: true }),
    (error) => error.name === 'AdapterError' && error.code === 'INVALID_REQUEST',
  );
});

test('profile constraints and text composition use application-facing camel case', async () => {
  const api = await loadCjs();
  const profile = await api.defineProfile({
    name: 'structured-note', version: '1', fields: [
      { name: 'note', fieldType: 'text', constraints: [{ kind: 'minimumLength', value: 2 }] },
    ],
    options: {
      textPipeline: {
        strategy: 'join_indented_continuations',
        normalization: { normalizePunctuation: false },
      },
    },
  });
  assert.deepEqual(profile.schema.fields[0].constraints, [{ kind: 'minimum_length', value: 2 }]);
  assert.equal(profile.schema.options.text_pipeline.normalization.normalize_punctuation, false);
  assert.equal(profile.schema.options.text_pipeline.normalization.trim_whitespace, true);
});

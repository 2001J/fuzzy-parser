import assert from 'node:assert/strict';
import { readFile, readdir } from 'node:fs/promises';
import { join } from 'node:path';
import test from 'node:test';
import {
  assertSourceReferences,
  fixture,
  loadCjs,
  loadEsm,
  repositoryRoot,
} from './helpers.mjs';

const profiles = [
  {
    file: 'attendance-profile.json',
    recordName: 'synthetic_attendance_entry',
    textFields: ['participant', 'contact_email', 'party_size', 'attending', 'note'],
    xlsxFields: ['participant', 'attending'],
  },
  {
    file: 'inventory-profile.json',
    recordName: 'synthetic_inventory_entry',
    textFields: ['item_label', 'supplier_email', 'units', 'available', 'stock_state', 'handling_note'],
    xlsxFields: ['item_label', 'units', 'available'],
  },
];

const inputs = [
  ['text', ['conformance', 'shared.txt'], undefined],
  ['txt', ['conformance', 'shared.txt'], 'shared.txt'],
  ['csv', ['conformance', 'shared.csv'], 'shared.csv'],
  ['xlsx', ['xlsx', 'sample.xlsx'], 'sample.xlsx'],
];

function records(response) {
  return response.content.records ?? response.content.sheets.flatMap((sheet) => sheet.records);
}

test('the installed runtime applies two caller profiles to one format corpus without identity dispatch', async () => {
  for (const load of [loadCjs, loadEsm]) {
    const api = await load();
    for (const profile of profiles) {
      const schema = JSON.parse(await fixture('conformance', profile.file));
      let sawUnused = false;
      for (const [format, path, filename] of inputs) {
        const bytes = await fixture(...path);
        const request = { input: { format, bytes, ...(filename ? { filename } : {}) }, schema };
        const first = await api.parse(request);
        const second = await api.parse(request);
        assert.equal(JSON.stringify(first), JSON.stringify(second));
        assert.equal(first.record_name, profile.recordName);
        assertSourceReferences(first);

        const parsed = records(first);
        assert.equal(parsed.length, 2);
        assert.deepEqual(
          parsed[0].parse.assignment.fields.map((field) => field.name),
          format === 'xlsx' ? profile.xlsxFields : profile.textFields,
        );
        assert.equal(parsed[0].parse.review.status, 'needs_review');
        assert.equal(parsed[1].parse.review.status, 'needs_review');
        assert(parsed[1].parse.assignment.warnings.some((warning) => warning.code === 'required_field_missing'));
        assert(parsed[1].parse.assignment.warnings.some((warning) =>
          warning.code === 'multiple_candidates_ambiguous' || warning.code === 'text_field_ambiguous'));
        assert(parsed[1].parse.assignment.unassigned_candidates.length > 0);
        sawUnused ||= first.source_evidence.blocks.some((block) =>
          block.unused_spans.some((span) => span.byte_start < span.byte_end));
      }
      assert.equal(sawUnused, true);
    }
  }
});

test('consumer profile names remain fixtures and are absent from engine and package implementation', async () => {
  const roots = [
    join(repositoryRoot, 'crates/parser-core/src'),
    join(repositoryRoot, 'crates/parser-formats/src'),
    join(repositoryRoot, 'crates/parser-schema/src'),
    join(repositoryRoot, 'packages/fuzzy-parser-node/src'),
    join(repositoryRoot, 'packages/fuzzy-parser-node/wasm/src'),
  ];
  let implementation = '';
  for (const root of roots) {
    for (const entry of await readdir(root, { withFileTypes: true })) {
      if (entry.isFile()) implementation += await readFile(join(root, entry.name), 'utf8');
    }
  }
  for (const profile of profiles) assert(!implementation.includes(profile.recordName));
  assert(!/QualEvents/i.test(implementation));
});

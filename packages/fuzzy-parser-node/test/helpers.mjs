import { createRequire } from 'node:module';
import { readFile } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

export const here = dirname(fileURLToPath(import.meta.url));
export const packageRoot = resolve(here, '..');
export const repositoryRoot = resolve(packageRoot, '../..');
export const require = createRequire(import.meta.url);

export const schemas = Object.fromEntries(
  await Promise.all(
    ['attendance-supported', 'inventory-supported'].map(async (name) => [
      name,
      JSON.parse(await readFile(join(repositoryRoot, 'fixtures/runtime', `${name}.json`), 'utf8')),
    ]),
  ),
);

export function fixture(...parts) {
  return readFile(join(repositoryRoot, 'fixtures', ...parts));
}

export function publicRequest(format, bytes, schema = schemas['inventory-supported'], filename) {
  return {
    input: { format, bytes, ...(filename === undefined ? {} : { filename }) },
    schema,
  };
}

export function loadCjs(root = packageRoot) {
  return require(join(root, 'dist/index.cjs'));
}

export function loadEsm(root = packageRoot) {
  return import(pathToFileURL(join(root, 'dist/index.mjs')).href);
}

export function assertSourceReferences(response) {
  const document = response.source_evidence.document;
  const records = response.content.records ?? response.content.sheets.flatMap((sheet) => sheet.records);
  for (const record of records) {
    const assignment = record.parse.assignment;
    const candidates = [
      ...record.parse.candidates,
      ...assignment.fields.flatMap((field) => field.candidates),
      ...assignment.unassigned_candidates,
    ];
    for (const candidate of candidates) {
      const reference = candidate.source_reference;
      if (!reference) continue;
      const block = document.blocks[reference.block_index];
      const rawKinds = new Set(['Text', 'DateTimeText', 'Duration', 'Error']);
      const text = block.value.kind === 'Null' ? '' : String(block.value.value);
      const bytes = Buffer.from(text, 'utf8');
      const { byte_start: start, byte_end: end } = reference.span;
      const boundary = (offset) => Buffer.from(bytes.subarray(0, offset).toString('utf8')).equals(bytes.subarray(0, offset));
      if (!Number.isInteger(start) || !Number.isInteger(end) || start < 0 || end < start || end > bytes.length || !boundary(start) || !boundary(end)) {
        throw new Error('invalid source reference boundary');
      }
      const expectedSpace = rawKinds.has(block.value.kind) ? 'raw_text_utf8' : 'rendered_value_utf8';
      if (reference.coordinate_space !== expectedSpace) throw new Error('invalid coordinate space');
      const resolved = bytes.subarray(start, end).toString('utf8');
      if (resolved !== candidate.raw_value) throw new Error('source reference did not resolve exact candidate');
    }
  }
}

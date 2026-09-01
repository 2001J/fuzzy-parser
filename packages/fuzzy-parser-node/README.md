# @fuzzy-parser/node

`@fuzzy-parser/node` is the domain-neutral Node.js WebAssembly boundary for
Fuzzy Parser. It accepts bytes, a declared format, and a caller-owned schema,
then returns the existing parse-response `0.1` object. Each call runs in its
own Worker; deadlines and `AbortSignal` cancel by terminating and reaping that
Worker. A deadline covers the complete call, including Worker/runtime startup;
it may therefore expire before parser entry on a slower host. `AbortSignal`
coverage separately proves termination after actual parser entry. The package
does not use a CLI fallback, network service, queue, or consumer-specific rules.

The package is implemented and locally pack-tested, but is not published by
this repository's CI.

## Application profiles

For repeated imports, define a caller-owned profile once instead of rebuilding
raw schema JSON for every parse:

```js
import { defineProfile, parseProfile, reviewRecords, unresolvedEvidence } from '@fuzzy-parser/node';

const profile = await defineProfile({
  name: 'contacts-import', version: '2026-08', recordName: 'contact',
  fields: [
    { name: 'person', fieldType: 'person_name', required: true, aliases: ['Name'] },
    { name: 'phone', fieldType: 'phone_number', required: true },
    { name: 'amount', fieldType: 'currency' },
    { name: 'notes', fieldType: 'text' },
  ],
});
const result = await parseProfile(profile, {
  format: 'csv', bytes: uploadedBytes, filename: 'contacts.csv',
});
const needsReview = reviewRecords(result);
const unresolved = unresolvedEvidence(result);
// unresolved.records contains unassigned candidates; unresolved.source keeps
// the canonical document and unused source spans for correction UIs.
```

`defineProfile` validates executable capabilities through the same Worker/WASM
compiler before application input is supplied. It does not infer business
meaning: field names, aliases, requiredness, enum values and constraints remain
application-owned. The same profile can parse text, TXT, CSV and XLSX even when
optional `amount` or `notes` fields are absent. Its application version is
separate from parser schema/result versions; use a new profile version for
meaning changes and retain prior versions for historical replays.

Profile fields and options use typed application-facing names. For example,
constraints use `{ kind: 'minimumLength', value: 2 }`; text normalization uses
`normalizePunctuation`, while the package translates these once to the stable
engine schema contract.

```js
import { parse, ParserFailure, AdapterError } from '@fuzzy-parser/node';

const result = await parse({
  input: {
    format: 'csv',
    bytes: await uploadedFile.bytes(),
    filename: 'contributors.csv', // optional basename metadata, never a path
  },
  schema: callerSchema,
}, {
  timeoutMs: 30_000,
  signal: abortController.signal,
});
```

Supported formats are `text`, `txt`, `csv`, and `xlsx`. `schema` may be the
schema `0.1` object or its exact JSON string. `options.tableSelection` exposes
the generic header, row, and sheet selectors documented by the engine.

Parser failures throw `ParserFailure` and retain the safe structured
`ErrorReport`. Package validation, asset, lifecycle, protocol, and output
failures throw `AdapterError` with one of the documented codes in
`dist/index.d.ts`. Results are never truncated. Default package guards are a
70 MiB request message, 64 KiB adapter options, 16 MiB result, 30 second
deadline, and 120 second maximum deadline; the engine's narrower typed #17
limits still apply and remain visible as `ParserFailure` reports.

The installed package verifies its adapter/parser/schema/contract versions,
the Rust source identity used for the build, and SHA-256 hashes of its generated
JS glue and WASM asset before parsing. Missing, corrupt, or mismatched assets
fail initialization without another backend.

Requires Node.js 22 or later. Local verification is:

```bash
node tools/ci/verify-node-package.mjs
```

That command builds the pinned WASM binding, tests CJS and ESM, packs and
installs the tarball into an isolated Node consumer, and builds/runs a generic
Next.js standalone fixture. It never publishes or deploys.

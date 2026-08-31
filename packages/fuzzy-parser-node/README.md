# @fuzzy-parser/node

`@fuzzy-parser/node` is the domain-neutral Node.js WebAssembly boundary for
Fuzzy Parser. It accepts bytes, a declared format, and a caller-owned schema,
then returns the existing parse-response `0.1` object. Each call runs in its
own Worker; deadlines and `AbortSignal` cancel by terminating and reaping that
Worker. The package does not use a CLI fallback, network service, queue, or
consumer-specific rules.

The package is implemented and locally pack-tested, but is not published by
this repository's CI.

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

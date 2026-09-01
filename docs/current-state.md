# Current State

This page is the concise capability matrix for the code on `development`.
Historical ticket evidence belongs in [internal documentation](internal/README.md).

Fuzzy Parser is a pre-1.0 Rust workspace at version `0.1.0`. Its Rust libraries,
CLI, and Node/WebAssembly package share the same parser core. The Node package is
pack/install tested but has not been published.

## Input support

| Input | Support | Important limits |
| --- | --- | --- |
| Pasted or inline text | Implemented | Text input is not automatically interpreted as CSV |
| Standard input | Implemented | Treated as text |
| UTF-8 TXT | Implemented | Non-UTF-8 input fails; byte and line limits apply |
| CSV | Implemented | Detects comma, semicolon, tab, and pipe delimiters |
| XLSX | Implemented | Reads stored/cached values; does not execute formulas or macros |
| Legacy XLS | Not implemented | Keep an application fallback when required |
| PDF, image, OCR | Not implemented | Outside the deterministic text/table engine |

CSV and XLSX support optional header, row, and sheet selection. The default
table path remains compatible with earlier behavior; explicit selection is
recommended when the caller knows the source layout.

## Field capabilities

| Field type | Support | Important limits |
| --- | --- | --- |
| `text` | Implemented | Assignment needs a matching header or literal caller label; residual text stays unresolved |
| `person_name` | Implemented | Possible-name evidence only; no identity or name dictionary |
| `phone_number` | Implemented | Conservative digit/separator matching; no country or locale interpretation |
| `email` | Implemented | Conservative ASCII pattern, not full RFC validation |
| `integer` | Implemented | Complete integer tokens |
| `decimal` | Implemented | Requires a decimal representation |
| `currency` | Implemented | Limited symbol-based parsing; not locale-aware money interpretation |
| `date` | Implemented | Conservative year-month-day forms |
| `datetime` | Not implemented | Profile compilation fails explicitly |
| `boolean` | Implemented | Fixed generic true/false aliases |
| caller-defined enum | Implemented | Single-token matching; ambiguous ownership remains unresolved |

Profiles support required and multiple fields, aliases, integer bounds, string
length bounds, enum values, and optional text composition. Unsupported options
or constraints fail during profile compilation instead of being silently
ignored.

## Text composition

Profiles may opt into:

- one block per record;
- indented continuation joining;
- splitting on caller-provided repeated identifiers.

The default path performs no hidden normalization pass. Composed records retain
mapping evidence back to the original blocks. Detection and label context do not
cross synthetic joins.

Arbitrary unlabeled prose is deliberately conservative. A string that could be
a name, note, or heading may remain unresolved rather than being assigned by
guesswork.

## Results

Implemented parse responses include:

- assigned fields and all detected candidates;
- unassigned candidates;
- record review status and reason codes;
- warnings;
- the canonical source document;
- source references for candidate copies;
- unused source spans;
- table and text-composition evidence where applicable.

Confidence values are deterministic heuristics, not accuracy probabilities.
Neither `clear` nor `needs_review` is business approval. See
[Results and review](results-and-review.md).

## Interfaces

| Interface | Status |
| --- | --- |
| `parser-api` reusable Rust profiles | Implemented in the workspace |
| Lower-level Rust crates | Implemented |
| `parser-cli` | Implemented |
| `@fuzzy-parser/node` CJS/ESM package | Implemented and pack/install tested; unpublished |
| Generic Next.js package consumption | Verified in an isolated fixture |
| HTTP service | Not implemented |
| Standalone graphical review/export tool | Not implemented |

The Node package uses one Worker-isolated WebAssembly backend. Deadlines and
`AbortSignal` terminate the per-call Worker; this is isolation and cleanup, not
cooperative cancellation inside synchronous parser code.

## Resource and privacy boundaries

Typed limits cover text, CSV, XLSX, schemas, record counts, and serialized
responses. Some checks necessarily occur after an underlying library has
materialized intermediate data, so the parser is not a hostile-file sandbox.

Default errors omit paths and arbitrary caller strings. Explicit diagnostics and
successful source-backed output may contain sensitive data and should not be
logged by default.

The parser does not:

- persist records;
- authorize users;
- enforce application duplicate or qualification policy;
- send messages;
- evaluate workbook code;
- archive exact uploaded files;
- automatically redact successful results.

## Verification status

CI runs on pull requests and pushes to `development` and `main`. It covers
formatting, Clippy, Rust tests/builds on Linux and macOS, dependency advisories,
CLI parity, Node/WASM packaging, generic Next.js consumption, WASM compilation,
and a non-published container smoke test.

The manual release workflow has not published a crate, npm package, container,
tag, or GitHub Release.

## What remains

Near-term capability work is summarized in the [roadmap](roadmap.md). The most
visible gaps are broader locale-aware values, datetime execution, legacy and
declared delimited formats, richer workbook display metadata, and a standalone
review/export experience.

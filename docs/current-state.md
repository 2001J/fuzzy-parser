# Current State

Last reviewed: 2026-08-30, including the independently reviewed and locally
integrated #14 text-composition and #16 table-selection extensions, the #11
runtime-boundary selection, the #10 source-evidence extension and the
independently verified [#21 Unicode context fix](https://github.com/2001J/fuzzy-parser/issues/21).
The local [#22 XLSX byte API](https://github.com/2001J/fuzzy-parser/issues/22)
implementation and file-reader parity have also been independently verified.
The local [#2 error migration](https://github.com/2001J/fuzzy-parser/issues/2) is
independently reviewed and verified with permanent privacy/compatibility regressions.
The [dated audit](audits/2026-08-27-backlog.md) records the earlier implementation baseline.

This document records only what is implemented in the repository now. It must not describe planned behavior as complete.

Resource-limit options now cover CSV bytes/rows/cells, XLSX compressed bytes,
sheets and extracted cells, schema bytes/fields/aliases/nesting, and parse
records and response bytes. Ordinary successful output remains unchanged.
XLSX expanded worksheet memory is checked only after calamine materializes a
range, and response limits are checked after constructing the response; these
limits are not a sandbox or preallocation guarantee.

## Repository state

The repository is a Rust workspace at version `0.1.0` using Rust edition 2024.

The workspace currently contains four crates:

- `parser-core`
- `parser-formats`
- `parser-schema`
- `parser-cli`

## Implemented today

- The workspace compiles as a multi-crate Rust project.
- `parser-core` provides serializable canonical raw-document models, source locations, raw values, warnings, structured parser errors, configurable derived text normalization, and deterministic record segmentation strategies including repeated-identifier splitting and heading-aware boundaries.
- `parser-core` detects conservative email, integer, decimal, phone-number, boolean, date, currency, and caller-defined enum field candidates with raw values, normalized values, heuristic confidence, reason codes, and byte spans in the detector's input text.
- Email detection recognizes whitespace and `, ; : ( ) [ ] < >` as token boundaries, preserving original UTF-8 byte spans and all unused punctuation/text. The #15 regression fix is independently reviewed and locally integrated, with core/CLI coverage of comma-adjacent and repeated addresses, Unicode prefixes, trailing punctuation, and rejected near misses.
- For scalar fields, `parser-core` assigns compatible candidates using canonical or caller-provided labels within a UTF-8-safe window of at most 40 preceding bytes, optional source-column metadata, or detected table-header labels, applies caller-provided integer and length constraints, selects the highest-confidence candidate when context is equal, preserves multiple values when requested preferring header-matching columns, and reports missing required fields, ambiguity, and unassigned candidates.
- The independently reviewed #13 implementation adds caller-directed multiword `text` and possible `person_name` fields after scalar/enum assignment. Literal labels and matching existing headers can direct ownership; residuals and competing values stay unresolved. Exact Unicode/interior whitespace and source references are preserved, and new assignments cannot reuse assigned intervals. See [semantics and migration](data-contracts.md#contextual-text-and-possible-person-names). Combined macOS/Linux and container verification passed; the issue is closed.
- `parser-core` groups blocks with row provenance into sheet rows, detects first-row headers using a heuristic, and exposes `parse_document_rows_with_assignment` for header-driven row assignment. Blocks without row provenance are excluded with warnings; the document-level response retains their values and an explicit exclusion reason.
- `parser-core` exposes `parse_text_with_assignment`, which composes the implemented detectors and assignment for one supplied text record. Its legacy behavior is unchanged. Schema-compiled plans may now opt into mapped normalization plus one-block, indented-continuation or caller-marker segmentation for text documents.
- `parse_document_with_assignment` chooses table rows when row provenance exists and otherwise parses each raw block separately. A compiled plan with no text-pipeline option follows that same branch before normalization, preserving the complete legacy response. Opt-in composed records expose the exact detector text, applied options, authoritative block indexes/raw memberships, source-less inserted newlines, monotone UTF-8 mapping runs and boundary evidence. `ParseResponse` embeds the unchanged canonical document, source metadata, coverage of parsed/header/excluded blocks, and unused spans. Candidate references resolve in every detected/assigned/unassigned copy. Input warnings are forwarded, and records carry deterministic draft/review reasons; see [data contracts](data-contracts.md).
- `parser-formats` reads UTF-8 TXT files, pasted text, standard input, and CSV files into canonical raw blocks while preserving content and source locations.
- [Permanent TXT fixture tests](../fixtures/text/README.md) cover Unicode/raw whitespace, empty input, consecutive blank lines, LF/CRLF and trailing terminators, invalid UTF-8, missing/directory paths, and injected permission-denied read propagation. Library tests assert complete raw documents and exact source byte slices; real-CLI tests materialize the same byte fixtures and independently assert canonical metadata, IDs, raw values, one-based lines, exact UTF-8 byte locations and structured failure output. Extraction behavior is unchanged.
- CSV extraction scores comma, semicolon, tab, and pipe delimiters, supports explicit overrides, quoted/multiline cells, empty cells, and row/column provenance.
- Opt-in CSV/XLSX companion readers return a non-wire `ExtractedTable` manifest. It retains original sheet order, empty sheets, original row inventory, blank CSV logical rows with quote-aware byte/line spans, exact document block indices, and detected merged regions as unsupported metadata without fabricating cells.
- `parser-formats` reads XLSX workbooks from paths or borrowed bytes with optional filename metadata, using one extraction path with sheet, row, column and typed-cell provenance. The byte API performs no filesystem/network I/O; both read stored/cached values without executing formulas or macros. See [XLSX library input](data-contracts.md#xlsx-library-input--implemented) for metadata and error semantics.
- `parser-schema` provides serializable generic target-schema models for fields, enum values, aliases, and basic constraints, plus structural validation for supported versions and ambiguous labels.
- `parser-schema` compiles executable schemas into a reusable core `ParsePlan`; CLI and Rust callers use the same detector/assignment pipeline. Strict execution JSON decoding checks unknown properties without changing structural schema validation. Enum values and aliases stay scoped to their field; unresolved ownership warns instead of choosing by schema order. See [capabilities and migration](data-contracts.md#executable-schema). This local #12 implementation is independently reviewed and verified.
- Format/schema errors share typed reports and safe default JSON/Display in `parser-core`. Explicit library diagnostics and a leading CLI `--diagnostics` expose only allowlisted context, which may be sensitive. The [error contract migration](data-contracts.md#error-contract-01-and-migration-from-unversioned-errors) preserves codes/cause meanings while changing default fields/messages, adding the separate error version and refining invalid-data I/O kinds. Successful output is unchanged.
- Text input has library-configurable byte and line-length limits. CLI TXT-file inspect/parse accepts trailing byte-limit and empty-policy overrides; defaults remain 1 MiB total, 64 KiB per line and empty acceptance. CSV, XLSX, schema and parsed-response paths now have typed library limits and safe defaults; the CLI applies those defaults without adding new flags. See the [resource-limit contract](data-contracts.md#resource-limits--implemented) and [exact TXT grammar](integration-strategy.md#cli-grammar-and-validation-options).
- The independently reviewed local [file-validation slice](file-validation.md) checks regular files, enabled extensions, metadata size and explicit empty policy, returning an opened handle. TXT paths use this helper and bounded reads on the same handle; default empty acceptance and successful raw output remain unchanged. The helper's CSV/XLSX eligibility does not integrate those readers.
- The CLI supports root/subcommand help, explicit TXT/CSV/XLSX path routing, `inspect --stdin`, `inspect --text <content>`, schema validation from path/stdin/text (compact output for files only), and positional file/stdin parsing with `--schema`. It validates the entire OS argument list before I/O, recognizes diagnostics only at the start, and preserves the 0/1/2 data/processing/usage boundary. Permanent subprocess tests cover grammar, precedence, TXT overrides and the full synthetic TXT fixture matrix; this does not establish wider engine readiness.
- The CLI `parse` command uses shared schema decoding/compilation and the versioned `ParseResponse` pipeline. `datetime` retains `schema_field_type_unsupported`; unsupported options, constraints and enum definitions fail explicitly using the existing safe error boundary. Historical unsupported text/name error payloads remain readable and render unchanged.
- CSV/XLSX path parsing accepts opt-in header, inclusive include/exclude row and XLSX sheet selectors. The fallible companion path emits table manifest evidence and typed `table_selection_error` failures; no-option parsing, inspection, TXT/text/stdin, and historical success output are unchanged.
- The corrected and independently reviewed [#11 WASM evaluation](evaluations/2026-08-30-wasm-runtime.md) exercises the shared byte/schema boundary through both CJS and ESM, checks exact native parity and source references, and demonstrates Worker entry/termination. It selects one Node WASM package with Worker isolation for #18. This remains local evaluation evidence, not an installable production adapter, public TypeScript API, true in-call cancellation, Vercel deployment proof or completion of #19.
- The `@fuzzy-parser/node` package implements that selected boundary with CJS/ESM entry points, TypeScript declarations, per-call Worker isolation, safe typed adapter failures, artifact identity checks, #17 limit preservation, and deadline/abort termination. Its tarball is installed into synthetic Node and generic Next.js standalone consumers during local verification. It is not published or deployed, and #19 selected-runtime independence remains open.
- The [GitHub Actions workflow](ci.md) defines test-only Rust quality, Linux/macOS tests, Node invocation parity, WASM library compilation, dependency advisory and container-semantic gates. It has no publication step; the first hosted run of this revision remains to be recorded in [#23](https://github.com/2001J/fuzzy-parser/issues/23).
- The CLI container is a batch artifact, not a selected runtime adapter or proven QualEvents deployment. Historical main-push image publication is removed in this revision; branches using the old workflow retain it until integration.
- The repository is licensed under Apache License 2.0.
- Permanent unit tests live in each crate's `tests/unit/mod.rs`; CLI subprocess tests remain in `tests/inspect.rs` and `tests/parse.rs`. Coverage includes the raw-model compatibility cases carried from #3, source resolution/unused content, typed values, warnings and old/new JSON golden contracts. [The acceptance audit](audits/2026-08-27-backlog.md) records the earlier temporary probes; passing current tests does not establish the missing contracts below.

## Known limitations

- CLI byte/empty overrides still apply only to TXT. CSV and XLSX file readers bound reads on the opened handle, but they do not provide an immutable file snapshot; metadata is only an initial size observation. Schema nesting is bounded before JSON materialization, while schema field/alias counts, CSV rows/cells, XLSX cells, parsed records and serialized response bytes are necessarily checked after their documented intermediate representation exists. See the [resource-limit contract](data-contracts.md#resource-limits--implemented).
- Raw in-process cause data and `Debug`, explicit diagnostics, and successful source-backed output remain potentially sensitive. Default error redaction does not add success-output redaction or a general diagnostics framework. The implemented resource limits bound the documented engine stages, but they are not a sandbox or a guarantee against every intermediate allocation.
- Normalization and segmentation are opt-in through `SchemaOptions.text_pipeline`; schemas without it retain independent raw-block text records. The composed path supports fixed-newline one-block, indented-continuation and caller-marker repeated-identifier strategies only. It deliberately does not expose table-row segmentation, production marker defaults or cross-segment candidates.
- Email detection retains its existing limited ASCII pattern and edge-period trimming; it is not full email syntax validation. Unsupported punctuation inside a token is not treated as a boundary. Other detectors keep their existing tokenization, and `--stdin` is text, not a tabular auto-detection mode.
- The legacy table path can still mistake an all-text first data row for a header. Callers must opt into #16 options to disable it, select an explicit row, or use bounded schema-informed search.
- Text/name fields require literal caller direction for assignment. Unlabeled names versus notes remain unresolved; typed cells are not coerced. The possible-name guard deliberately rejects whole scalar tokens such as `No`; it is not a name dictionary or identity check. In opt-in composed text, labels, residual regions and detectors stay within each source segment; normalization may aid structure but returned text/name values remain exact original substrings.
- Legacy candidate spans in a table still refer to concatenated, trimmed row text. New source references index stored strings or explicitly rendered typed values in the embedded canonical document, not original CSV/XLSX file bytes. Legacy extraction still omits blank CSV physical lines; the opt-in companion inventories blank logical rows but neither path preserves original quoting, TXT line terminators or complete workbook metadata. Exact-file retention remains the caller's responsibility.
- Executable schemas support permissive `allow_unknown_fields=true`; false is explicitly unsupported. Enum matching still uses single whitespace-separated tokens, with multiword canonical output possible through short aliases. Ambiguous lexical ownership remains unassigned, and even context-resolved alternative hypotheses remain visible for review. Locale/country hints, uniqueness, expected-column settings and runtime limits are not executable schema options. Library `unique` behavior is not a database duplicate policy.
- XLSX reads stored/cached values within worksheet ranges. Formula origin/evaluation, date interpretation, display formatting, styles/colors and business-ready selection are unsupported; merged regions are only reported as uninterpreted metadata. Legacy `.xls` is unsupported.

## Not implemented yet

The following capabilities are planned but do not exist yet:

- Datetime field execution and broader locale-aware field interpretation.
- A unified serialized parse request; the reusable Rust schema compiler and core plan are available separately.
- Aggregate record confidence and statistics. Current draft/review statuses expose generic evidence gaps only; heuristic scores are not calibrated accuracy probabilities. Business rejection/approval remains host-owned, not a planned engine capability.
- The selected installable Node WASM adapter and public TypeScript contract. The isolated #11 binding and Node harness are evaluation tooling only; native Node bindings and HTTP services are not selected alternatives.
- A standalone graphical interface.
- The cross-profile, no-QualEvents independence gate described in [testing strategy](testing-strategy.md#cross-profile-conformance-and-independence--planned).
- Parser-owned export to CSV, XLSX, or clipboard templates (QualEvents has its own export behavior).
- OCR or PDF support.

## Current verification commands

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace
```

## Planned work

See the [roadmap](roadmap.md) for the first implementation ticket and dependency
order, and [integration strategy](integration-strategy.md) for the QualEvents
handoff. Neither plan is implemented behavior.

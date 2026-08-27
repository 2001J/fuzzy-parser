# Current State

Last reviewed: 2026-08-28, including the #10 source-evidence extension and the
independently verified [#21 Unicode context fix](https://github.com/2001J/fuzzy-parser/issues/21).
The [dated audit](audits/2026-08-27-backlog.md) records the earlier implementation baseline.

This document records only what is implemented in the repository now. It must not describe planned behavior as complete.

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
- `parser-core` assigns compatible candidates to caller-provided fields, uses canonical or caller-provided labels within a UTF-8-safe window of at most 40 preceding bytes, optional source-column metadata, or detected table-header labels as assignment context, applies caller-provided integer and length constraints, selects the highest-confidence candidate when context is equal, preserves multiple values when requested preferring header-matching columns, and reports missing required fields, ambiguity, and unassigned candidates.
- `parser-core` groups blocks with row provenance into sheet rows, detects first-row headers using a heuristic, and exposes `parse_document_rows_with_assignment` for header-driven row assignment. Blocks without row provenance are excluded with warnings; the document-level response retains their values and an explicit exclusion reason.
- `parser-core` exposes `parse_text_with_assignment`, which composes the implemented detectors and assignment for one supplied text record. Normalization and segmentation are separate library APIs, not stages used by this function.
- `parse_document_with_assignment` chooses table rows when row provenance exists and otherwise parses each raw block separately. `ParseResponse` embeds the unchanged canonical document, source metadata, coverage of parsed/header/excluded blocks, and unused spans. Candidate references resolve in every detected/assigned/unassigned copy. Input warnings are forwarded, and records carry deterministic draft/review reasons; see [data contracts](data-contracts.md).
- `parser-formats` reads UTF-8 TXT files, pasted text, standard input, and CSV files into canonical raw blocks while preserving content and source locations.
- CSV extraction scores comma, semicolon, tab, and pipe delimiters, supports explicit overrides, quoted/multiline cells, empty cells, and row/column provenance.
- `parser-formats` reads XLSX workbooks with sheet, row, column, and typed-cell provenance; it reads stored values only and does not execute formulas or macros.
- `parser-schema` provides serializable generic target-schema models for fields, enum values, aliases, and basic constraints, plus structural validation for supported versions and ambiguous labels.
- Text input has library-configurable byte and line-length limits; the CLI uses the fixed defaults of 1 MiB total and 64 KiB per line. Empty text is accepted. CSV, XLSX, and schema loading do not have equivalent configurable resource limits.
- The CLI supports help output, `inspect <path>` for TXT, CSV, and XLSX files, `inspect --stdin`, `inspect --text <content>`, schema validation from a path, standard input, or inline text with optional compact output, and `parse <path> --schema <schema-path>` / `parse --stdin --schema <schema-path>`, emitting canonical JSON with structured errors and nonzero exit codes.
- The CLI `parse` command loads a validated caller schema, converts supported field types into assignment instructions, and runs the versioned `ParseResponse` pipeline; schemas that reference not-yet-supported field types (`text`, `person_name`, `datetime`) are rejected with a structured `schema_field_type_unsupported` error instead of silently dropping fields.
- GitHub Actions runs formatting, Clippy, tests, a workspace build, and CLI-container build/smoke checks on pull requests and pushes to `main`.
- The CLI container is the current deployable batch artifact; pushes to `main` publish its `latest` image to GHCR.
- The repository is licensed under Apache License 2.0.
- Permanent unit tests live in each crate's `tests/unit/mod.rs`; CLI subprocess tests remain in `tests/inspect.rs` and `tests/parse.rs`. Coverage includes the raw-model compatibility cases carried from #3, source resolution/unused content, typed values, warnings and old/new JSON golden contracts. [The acceptance audit](audits/2026-08-27-backlog.md) records the earlier temporary probes; passing current tests does not establish the missing contracts below.

## Known limitations

- Unknown file extensions fall through to the TXT reader; the shared file-validation API, explicit empty-file policy, and strict dispatch are incomplete (#5/#6 in the [roadmap](roadmap.md)).
- Error JSON and messages can expose supplied absolute paths. Schema errors use a separate CLI envelope; not all errors are covered by exact serialization tests.
- `parse` does not invoke normalization or record segmentation. Indented continuation lines still become separate records.
- Plain-text detectors use conservative token matching; comma-adjacent email can be missed. `--stdin` is text, not a tabular auto-detection mode.
- The table header heuristic can mistake an all-text first data row for a header. There are no public CLI options for header/row/sheet selection.
- Legacy candidate spans in a table still refer to concatenated, trimmed row text. New source references index stored strings or explicitly rendered typed values in the embedded canonical document, not original CSV/XLSX file bytes. Extraction still omits original quoting, blank CSV physical lines, TXT line terminators and some workbook metadata; exact-file retention remains the caller's responsibility.
- CLI schema conversion pools enum definitions across fields and ignores `allow_unknown_fields`. Locale/country hints and expected-column settings are not part of the current executable schema interface. Library `unique` behavior is not a database duplicate policy.
- XLSX reads stored values within worksheet ranges. Date serials, style-based selection, and displayed formatting are not a complete consumer import contract. Legacy `.xls` is unsupported.

## Not implemented yet

The following capabilities are planned but do not exist yet:

- Additional field candidate detection beyond email, integer, decimal, phone-number, boolean, date, currency, and caller-defined enum values, including residual text and person-name fields.
- A reusable schema-to-engine entry point outside the CLI and a unified serialized parse request.
- Aggregate record confidence and statistics. Current draft/review statuses expose generic evidence gaps only; heuristic scores are not calibrated accuracy probabilities. Business rejection/approval remains host-owned, not a planned engine capability.
- TypeScript, WebAssembly, native Node, or HTTP integration.
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

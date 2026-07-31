# Roadmap

This roadmap defines implementation order, not guaranteed dates. Each release should produce a working vertical slice before the next layer is added.

## 0.1 — Workspace foundation

Status: complete.

- Rust workspace.
- Four initial crates.
- Formatting, Clippy, tests, and build checks in CI.
- Apache 2.0 license.
- Project documentation baseline.

## 0.2 — TXT inspection path

Goal: prove the first complete source-to-JSON path.

- Structured errors.
- Canonical `RawDocument`, `RawBlock`, and source-location models.
- UTF-8 TXT validation and reading.
- One raw block per source line.
- CLI `inspect` command.
- JSON output.
- Fixture-backed tests and CLI end-to-end tests.

The parser must still make no fuzzy interpretation in this release.

## 0.3 — Pasted text and input dispatch

- Raw text input.
- Standard input.
- Unified input dispatcher.
- Equivalent canonical output for pasted and uploaded text.
- Resource limits for text size and line length.

## 0.4 — CSV extraction

- CSV adapter.
- Comma, semicolon, tab, and pipe delimiter scoring.
- Explicit delimiter override.
- Quoted and multiline cells.
- Row and column provenance.
- Clean and deliberately messy CSV fixtures.

## 0.5 — XLSX extraction

- Workbook inspection.
- Sheet metadata.
- Cell extraction with row, column, and sheet provenance.
- Numeric, text, date, blank, formula-result, and merged-cell handling.
- No macro or formula execution.

## 0.6 — Normalization

- Normalized block model.
- Whitespace and punctuation normalization.
- Recorded transformations.
- Noise marking for list prefixes, headings, timestamps, and sender prefixes.
- Raw source preservation.

## 0.7 — Record segmentation

- Record candidate model.
- One-line and one-row strategies.
- Multiline continuation heuristics.
- Multiple-records-per-line heuristics.
- Boundary confidence and reasons.

## 0.8 — Schema contract

- Versioned target schema JSON.
- Generic field types.
- Required and optional fields.
- Enum values and aliases.
- Locale and caller hints.
- Schema validation.
- CLI schema loading.

## 0.9 — Candidate detection

- Phone.
- Email.
- Integer and decimal.
- Currency.
- Date and datetime.
- Boolean.
- Enum alias.
- Residual text and conservative person-name candidates.
- Source spans and candidate confidence.

## 0.10 — Assignment and validation

- Type-compatible assignment.
- Label and header context scoring.
- Position and uniqueness scoring.
- Required-field warnings.
- Multiple-candidate ambiguity.
- Unassigned candidate reporting.
- Caller-provided validation constraints.

## 0.11 — Explainable parse result

- Layered confidence.
- Stable reason codes.
- Record statuses.
- Rejected fragments.
- Statistics.
- Versioned public parse result contract.
- Golden JSON tests.

## 0.12 — Standalone review tool

- TypeScript interface.
- Paste and upload controls.
- Custom schema editor.
- Review table.
- Source evidence.
- Edit, approve, reject, split, and merge actions.
- JSON, CSV, and clipboard export.

The first UI integration may use a local service or CLI bridge before WebAssembly.

## 0.13 — TypeScript and WebAssembly

- Stable TypeScript request and response types.
- WebAssembly build.
- Browser-side text parsing.
- Shared fixtures proving CLI and WebAssembly parity.
- npm package preparation.

## 0.14 — Reliability and performance

- Property-based tests.
- Fuzz targets.
- Resource-limit coverage.
- Benchmarks by stage and input size.
- Memory profiling for large tables.

## Later candidates

- XLSX export.
- Duplicate candidate detection.
- Saved parser profiles.
- Text-based PDF extraction.
- OCR adapter.
- Native Node binding.
- HTTP service.
- Optional correction-learning system.

These should not interrupt the deterministic text/table path unless a concrete user requirement changes priorities.

## Milestone rule

Do not begin the next release merely because code for the current one exists. The current release must have:

- Passing tests.
- Documented public behavior.
- Structured failure behavior.
- A usable end-to-end demonstration.
- No known silent data loss.

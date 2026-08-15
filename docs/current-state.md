# Current State

Last reviewed: 2026-07-31.

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
- `parser-core` detects conservative email, integer, decimal, phone-number, boolean, and date field candidates with raw values, normalized values, confidence, reason codes, and byte-accurate source spans.
- `parser-formats` reads UTF-8 TXT files, pasted text, standard input, and CSV files into canonical raw blocks while preserving content and source locations.
- CSV extraction scores comma, semicolon, tab, and pipe delimiters, supports explicit overrides, quoted/multiline cells, empty cells, and row/column provenance.
- `parser-formats` reads XLSX workbooks with sheet, row, column, and typed-cell provenance; it reads stored values only and does not execute formulas or macros.
- `parser-schema` provides serializable generic target-schema models for fields, enum values, aliases, and basic constraints, plus structural schema validation.
- `parser-formats` exposes configurable default-safe byte and line-length limits for text input.
- The CLI supports `inspect <path>` for TXT, CSV, and XLSX files, `inspect --stdin`, and `inspect --text <content>`, emitting the canonical raw document as JSON with structured errors and nonzero exit codes.
- GitHub Actions runs formatting, Clippy, tests, and a workspace build on pull requests and pushes to `main`.
- The repository is licensed under Apache License 2.0.
- The root README describes the intended workspace boundaries and local validation commands.

## Not implemented yet

The following capabilities are planned but do not exist yet:

- Additional field candidate detection beyond email, integer, decimal, phone-number, boolean, and date values.
- CLI schema loading and caller-provided schema parsing.
- Candidate-to-field assignment.
- Confidence scoring or explanations.
- Structured warnings or rejected fragments.
- General parse request/response contracts beyond raw-document inspection.
- TypeScript, WebAssembly, native Node, or HTTP integration.
- A standalone graphical interface.
- Export to CSV, XLSX, or clipboard templates.
- OCR or PDF support.

## Current verification commands

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace
```

## Immediate next slice

The next implementation slice should extend candidate detection to phone, decimal, date, boolean, and caller-defined enum values before schema-driven assignment.

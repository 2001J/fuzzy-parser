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
- `parser-core` provides serializable canonical raw-document models, source locations, raw values, warnings, and structured parser errors.
- Each library crate contains a minimal readiness function and a placeholder unit test.
- The CLI binary prints `parser-cli ready` and contains a placeholder unit test.
- GitHub Actions runs formatting, Clippy, tests, and a workspace build on pull requests and pushes to `main`.
- The repository is licensed under Apache License 2.0.
- The root README describes the intended workspace boundaries and local validation commands.

## Not implemented yet

The following capabilities are planned but do not exist yet:

- Pasted-text or standard-input ingestion.
- TXT file validation or extraction.
- CSV delimiter detection or extraction.
- XLSX workbook inspection or extraction.
- Text normalization.
- Record segmentation.
- Field candidate detection.
- Caller-provided schema parsing and validation.
- Candidate-to-field assignment.
- Confidence scoring or explanations.
- Structured warnings or rejected fragments.
- A real CLI command contract.
- JSON request or response contracts.
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

The next implementation slice should establish the first end-to-end path:

1. Define structured errors.
2. Define the canonical `RawDocument` and source-location models.
3. Read a UTF-8 `.txt` file without normalization.
4. Expose the result through a CLI inspection command as JSON.
5. Add fixture-backed unit and CLI integration tests.

Until this slice is complete, the project should not claim to parse messy data. It is currently a workspace foundation for that parser.

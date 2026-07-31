# Architecture

## Overview

Fuzzy Parser is organized as a reusable Rust engine with thin integration surfaces around it.

```text
Input source
    ↓
Format adapter
    ↓
Canonical raw document
    ↓
Normalization
    ↓
Record segmentation
    ↓
Field candidate detection
    ↓
Schema-driven assignment
    ↓
Validation, confidence, and warnings
    ↓
Parse result
    ↓
CLI / WebAssembly / service / standalone UI
```

The architecture separates source extraction from interpretation. A CSV adapter knows how to read cells and coordinates; it does not know what a customer, guest, amount, or status means.

## Workspace responsibilities

### `parser-core`

Owns generic parsing behavior and shared runtime models:

- Canonical raw and normalized document models.
- Source locations and text spans.
- Normalization transforms.
- Record candidates and segmentation strategies.
- Generic field candidates.
- Candidate assignment.
- Parse orchestration.
- Confidence components and explanations.
- Warnings, rejected fragments, and parse statistics.

It must not depend on the CLI or a product-specific profile.

### `parser-formats`

Owns source-specific extraction:

- Pasted text and standard input.
- TXT.
- CSV and delimiter detection.
- XLSX workbook and cell extraction.
- Future PDF-text or OCR adapters.

The output of every adapter is a canonical document. Adapters preserve source metadata and must not overwrite raw values with normalized values.

### `parser-schema`

Owns the caller-provided description of desired output:

- Schema document and version.
- Field definitions and types.
- Required and optional fields.
- Enum values and aliases.
- Locale and country hints.
- Caller-provided labels, stop words, and constraints.
- Schema validation.

The schema crate describes generic structure. Product-specific schemas live in consuming applications or external profile files.

### `parser-cli`

Owns command-line concerns:

- Argument parsing.
- Reading file paths and stdin.
- Loading schema JSON.
- Selecting output mode.
- Rendering JSON and human-readable diagnostics.
- Stable exit codes.

The CLI should call library APIs rather than reproduce parsing logic.

## Dependency direction

The intended direction is:

```text
parser-cli ───────┐
                  ├──> parser-core
parser-formats ───┤
                  └──> parser-schema where required
```

Exact dependencies may evolve, but these constraints remain:

- `parser-core` must not depend on `parser-cli`.
- Format adapters must not depend on business applications.
- The CLI must not become the only home of public models.
- Circular crate dependencies are not allowed.

If shared request or response models are needed by several crates, place them at the lowest stable layer rather than introducing a broad utility crate prematurely.

## Processing boundaries

### Extraction boundary

Input adapters may:

- Validate source-level limits.
- Decode supported formats.
- Preserve lines, cells, sheets, rows, columns, and file metadata.
- Produce structured extraction warnings.

They may not:

- Assign values to business fields.
- Apply invitation, payment, or product-specific meaning.
- Delete data merely because it looks irrelevant.

### Core parsing boundary

The core may:

- Normalize equivalent text representations.
- Propose record boundaries.
- Detect generic candidate types.
- Assign candidates to caller-defined fields.
- Return confidence, explanations, and warnings.

It may not:

- Create database records.
- Trigger downstream messages or exports.
- Hide unresolved ambiguity.
- Assume a specific business workflow.

### Host application boundary

A consuming application may:

- Provide schemas and domain profiles.
- Display review interfaces.
- Apply domain rules.
- Accept, reject, or edit parsed values.
- Save confirmed records.
- Trigger application-specific actions after confirmation.

## Deployment shapes

### CLI-first

The first complete surface is a CLI because it gives the Rust project an independent executable contract and supports fixture-driven end-to-end tests.

### WebAssembly

A future WebAssembly package can run parsing in a browser. This is attractive for low latency and privacy because input can remain local until a user confirms results.

### Native binding

A native Node binding may be introduced for server-side TypeScript applications when benchmarks justify the packaging complexity.

### HTTP service

A service can make the parser language-independent and centrally versioned, but adds deployment, authentication, observability, and data-transfer concerns. It should not be the first integration.

## Architectural invariants

- Original input remains traceable.
- All public outputs are serializable.
- Parsing is deterministic for the same input, schema, options, and parser version.
- Every stage can be tested independently.
- Unused and rejected content remains observable.
- Domain meaning is injected by the caller.
- Resource limits are configurable and safe by default.
- Input is treated as untrusted.

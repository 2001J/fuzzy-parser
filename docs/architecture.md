# Architecture

## Overview

Fuzzy Parser is organized as a reusable Rust engine with thin integration surfaces around it.
The diagram and responsibilities below describe the target architecture, not a
claim that every stage is composed today. The [pipeline](parsing-pipeline.md)
identifies currently reachable paths; [current state](current-state.md) lists gaps.
QualEvents is the first validation consumer, not a dependency or source of core
domain types. Consumer ownership is defined in
[integration strategy](integration-strategy.md).

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
- Shared typed failures, versioned error payloads and safe message rendering.

It must not depend on the CLI or a product-specific profile. Consumer names,
identifiers, schemas and domain constants must not select special engine behavior.

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

The current local-crate dependencies, verified from the manifests, are:

```text
parser-cli → parser-formats → parser-core
parser-cli → parser-core
parser-cli → parser-schema
parser-schema → parser-core
```

Exact dependencies may evolve, but these constraints remain:

- `parser-core` must not depend on `parser-cli`.
- Format adapters must not depend on business applications.
- The CLI must not become the only home of public models.
- Circular crate dependencies are not allowed.

If shared request or response models are needed by several crates, place them at the lowest stable layer rather than introducing a broad utility crate prematurely.
The schema-to-core dependency provides the shared error boundary; schema cause
variants and validation stay in `parser-schema`. No dependency cycle or separate
CLI error model is needed. [Error contracts](data-contracts.md#error-contract-01-and-migration-from-unversioned-errors)
define the migration. This dependency does not implement schema compilation.
Today `assignment_spec` in the CLI converts schema fields into core assignment
instructions. [#12](https://github.com/2001J/fuzzy-parser/issues/12) moves that
interpretation into a shared library boundary; the diagram must be updated if
the dependency direction changes.

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

The implemented CLI and batch container provide an independent executable
surface. No WebAssembly, native Node, or HTTP interface exists yet.

Following [ADR 0005](decisions/0005-independent-engine-consumer-validation.md),
[ADR 0006](decisions/0006-library-interface-runtime-evaluation.md) proposes a
library caller interface without queues or a separate service. Node/CLI
evaluation tooling and a WASM compilation check exist; backend selection,
production packaging and deployment verification remain open. The bounded evidence
has passed independent review. The [integration strategy](integration-strategy.md) links it
and remaining gates; other transports are not prerequisites. The adapter may
target a runtime without depending on the consumer. It must invoke the same
engine/schema APIs and work with unrelated caller profiles with QualEvents absent.

## Architectural invariants

These are requirements. Canonical source evidence is now embedded in document
parse responses; uniform resource limits and broader extraction fidelity remain
incomplete. See [current state](current-state.md).

- Original input remains traceable.
- All public outputs are serializable.
- Parsing is deterministic for the same input, schema, options, and parser version.
- Every stage can be tested independently.
- Unused and rejected content remains observable.
- Domain meaning is injected by the caller.
- Resource limits are configurable and safe by default.
- Input is treated as untrusted.

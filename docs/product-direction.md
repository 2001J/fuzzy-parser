# Product Direction

## Product statement

Fuzzy Parser is an independent, domain-neutral engine for converting messy human-created text and tabular input into reviewable structured records.

It is intended for situations where people paste or upload inconsistent data instead of providing a clean database export: copied chat lists, improvised spreadsheets, CSV files with shifting columns, and plain-text records that use mixed delimiters or incomplete formatting.

## Core user outcome

A user or consuming application should be able to provide:

1. Raw input.
2. A description of the fields it wants.
3. Optional locale, aliases, constraints, and parsing hints.

The parser should return:

- Candidate records.
- Candidate field values.
- Normalized values where safe.
- Confidence and explanations.
- Warnings and unresolved ambiguity.
- Source locations for every extracted value.
- Rejected or unused fragments instead of silently losing them.

The result is a draft for review, not an unquestionable truth.

## Independent product and embedded engine

The project has two valid product shapes that must share one parser core.
The first delivery priority is a **reviewable, independently usable import
engine**, validated through its first real consumer, QualEvents. That consumer
does not define the engine's identity, domain model, public contract, or
dependencies. Standalone review tooling is later; library/CLI use stays supported.

### Standalone tool

A user pastes text or uploads a supported file, defines or selects an output schema, reviews the structured preview, corrects uncertain values, and exports the result.

### Embedded engine

Any application supplies uploaded/pasted input and its schema/options. No
consumer-specific constants, schemas, identifiers, imports, or dependencies
belong in generic engine behavior. Examples for consumers may exist as isolated
synthetic fixtures, never as compiled-in profiles or conditional domain rules.

QualEvents intends to use the engine for its supported text and tabular import
processing, not just optional pasted-text assistance. Its Event/Guest/Contributor
concepts and profiles remain in the host. Its adoption and cutover are separate
from completion of generic engine capabilities.

The consuming application owns:

- Business terminology.
- Permissions, business scope, duplicate policy, and qualification.
- Domain-specific validation.
- Review workflow.
- Corrections, export, and the decision to confirm an import.
- Persistence.
- Messaging or downstream side effects.
- Whether a warning blocks an import.

The parser owns generic extraction, normalization, segmentation, candidate
detection, assignment, uncertainty, and provenance. The
[integration strategy](integration-strategy.md) defines the reusable boundary
and the separate first-consumer handoff.

### Verified independence gate

Independence is demonstrated across unrelated caller profiles with the first
consumer absent. The authoritative [acceptance gate](testing-strategy.md#cross-profile-conformance-and-independence--implemented)
and [capability matrix](conformance.md) are completed in
[#19](https://github.com/2001J/fuzzy-parser/issues/19) for the implemented
capability set.

## Initial target inputs

The initial target sources are listed below. Adapters exist today, but complete
review/integration readiness is not implied; see [current state](current-state.md).

- Pasted multiline text.
- Standard input.
- UTF-8 `.txt` files.
- `.csv` files with common delimiter detection.
- `.xlsx` workbooks with source coordinates preserved.

Future adapters may include text-based PDF and OCR, but those are separate extraction concerns and must not delay the deterministic core.

## Initial target fields

The schema system should eventually support generic field types such as:

- Free text.
- Person name candidate.
- Phone number.
- Email.
- Integer and decimal.
- Currency.
- Date and time.
- Boolean.
- Enum with caller-provided aliases.

The parser must not infer domain meaning that the caller did not supply.

## Product principles

### Preserve before interpreting

Raw input must remain available even after normalization and assignment.

### Admit uncertainty

Unknown, ambiguous, missing, conflicting, and low-confidence states are valid outputs.

### Explain results

A field assignment should expose the evidence that caused it: pattern match, label proximity, schema alias, position, uniqueness, or caller configuration.

### Be schema-driven, not domain-fixed

Business assumptions are supplied by the caller. The parser remains reusable across products.

### Review before side effects

The parser produces structured drafts. It does not send messages, create production records, charge money, or generate access credentials.

### Deterministic first

Start with deterministic rules and measurable heuristics. Machine learning or LLM assistance may later be optional, but must not become required for basic operation.

### Fast enough, then measured

Rust is chosen for a safe reusable core and future native/WebAssembly integration. Performance work should follow benchmarks rather than assumptions.

## Explicit non-goals for the initial releases

- Perfect interpretation of arbitrary documents.
- Supporting every file type.
- Replacing human review.
- Automatically learning from private data without an explicit design.
- Executing spreadsheet formulas or macros.
- Domain-specific guest, pledge, inventory, or payment logic in the parser core.
- Generating production side effects from parser output.
- OCR, scanned PDF, or image understanding before the text and table pipeline is reliable.

## Long-term direction

The parser may eventually be distributed as:

- A Rust library.
- A CLI binary.
- A WebAssembly/npm package for TypeScript applications.
- A native Node binding where server-side performance justifies it.
- An HTTP service for language-independent integration.
- A standalone review application.

All surfaces should converge on the same versioned request and response contracts.
They are alternatives to evaluate, not a sequence of prerequisites. The
[roadmap](roadmap.md) schedules one reusable boundary and retains
broader consumers, standalone tooling, and PDF/OCR as later work.

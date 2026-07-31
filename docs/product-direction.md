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

The project has two valid product shapes that must share one parser core:

### Standalone tool

A user pastes text or uploads a supported file, defines or selects an output schema, reviews the structured preview, corrects uncertain values, and exports the result.

### Embedded engine

Another application supplies its own schema and workflow. For example, an event platform may define guest or contribution profiles, but those profiles remain outside the generic parser core.

The consuming application owns:

- Business terminology.
- Domain-specific validation.
- Review workflow.
- Persistence.
- Messaging or downstream side effects.
- Whether a warning blocks an import.

The parser owns generic extraction, uncertainty, provenance, and schema-driven structuring.

## Initial target inputs

The first supported sources should be:

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

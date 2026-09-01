# Product Direction

## Product statement

Fuzzy Parser converts inconsistent human-created text and tabular input into
structured drafts that remain traceable to their source.

It is for applications whose users paste lists, upload improvised spreadsheets,
or receive files with changing labels and incomplete values. The parser should
arrange what it can, identify what it cannot safely interpret, and preserve
everything needed for review.

## User outcome

A person should be able to paste or upload data without understanding parser
schemas. The consuming application selects a reusable profile and presents the
result as:

- proposed records and fields;
- normalized values where safe;
- concise review reasons;
- unresolved or unused content;
- source evidence for every suggestion;
- copy/export/correction options before confirmation.

The parser produces a draft, not unquestionable truth.

## Independent engine

Fuzzy Parser is a domain-neutral engine with several interfaces:

- a Rust application API;
- lower-level Rust crates;
- a command-line tool;
- a Node/WebAssembly package;
- a possible future standalone review application.

All interfaces use the same core. Consumer-specific concepts, schemas,
identifiers, and dependencies do not belong in parser behavior.

QualEvents is the first real consumer and a useful validation case. It does not
define the engine. Its profiles, business rules, review screens, exports,
confirmation, persistence, and messaging stay in the host application.

## Product principles

### Profiles belong to applications

An application developer defines and versions its vocabulary once. End users
paste or upload data; they do not rebuild a schema for every import.

### Preserve before interpreting

Raw input and source references remain available after extraction,
normalization, assignment, and review.

### Admit uncertainty

Missing, ambiguous, conflicting, and unresolved values are valid outputs. The
parser should abstain rather than manufacture certainty.

### Explain suggestions

Assignments expose reasons and source evidence. Heuristic scores are not
calibrated accuracy probabilities.

### Review before side effects

Parsing does not create records, send messages, charge money, or authorize a
workflow. Consuming applications validate and explicitly confirm drafts.

### Deterministic first

Deterministic rules and measurable heuristics come before optional machine
learning or LLM assistance. Basic operation must not depend on a remote model.

### One core, simple integration

Applications should call a library, not operate a queue or separate service
unless a future cross-language need clearly justifies one.

## Product boundaries

The parser owns:

- format extraction;
- canonical values and provenance;
- normalization and record segmentation;
- candidate detection and schema assignment;
- generic constraints;
- warnings, review reasons, and unused evidence.

The consuming application owns:

- authorization and business scope;
- field meaning and profile selection;
- duplicate and qualification policies;
- correction and approval workflows;
- exports and persistence;
- messaging and downstream effects.

## Near-term direction

The immediate product work is to make real messy-list review easier without
weakening provenance or uncertainty:

- broader locale-aware phone, currency, date, and datetime interpretation;
- declared TSV and delimited-text handling;
- richer workbook display and sheet metadata;
- reusable profile examples across unrelated domains;
- a clear standalone review/export experience built on the same contracts.

See [Current state](current-state.md) for implemented behavior and
[Roadmap](roadmap.md) for sequencing.

## Non-goals

- Perfect interpretation of arbitrary documents.
- Silent automatic approval.
- Domain-specific guest, pledge, inventory, or payment rules in the core.
- Executing spreadsheet formulas, macros, or external links.
- OCR or image understanding before deterministic text/table behavior is solid.
- Automatic learning from private data without an explicit privacy design.

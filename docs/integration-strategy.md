# Integration Guide

This guide explains how an application should call Fuzzy Parser. Exact wire
shapes live in [Data contracts](data-contracts.md); supported capabilities live
in [Current state](current-state.md).

## Integration model

```text
application-owned profile
        +
uploaded or pasted input
        |
        v
Fuzzy Parser
  extraction -> detection -> assignment -> review evidence
        |
        v
structured draft
        |
        v
application-owned correction, validation, export, and confirmation
```

The application defines a profile once and chooses it for an import workflow.
The end user supplies data and reviews the result. They should not have to write
schema JSON every time they paste a list.

## Responsibilities

| Fuzzy Parser | Consuming application |
| --- | --- |
| Input adapters and canonical source | Upload/paste controls |
| Generic field detection and assignment | Profile selection and business vocabulary |
| Constraints supported by the engine | Domain validation and duplicate policy |
| Warnings, review reasons, and source evidence | Review/correction UI |
| Deterministic draft output | Authorization, confirmation, persistence, and messaging |

Parsing must not create business records or downstream side effects.

## Recommended application flow

1. Define and version a profile at application startup or configuration time.
2. Accept bytes or pasted text from an authorized user.
3. Call the parser with that profile and explicit input format.
4. Validate the response contract and parser version.
5. Show assigned fields beside source and unresolved evidence.
6. Let the user correct, omit, or resolve values.
7. Re-run application validation and authorization.
8. Require explicit confirmation before persistence.
9. Save through existing application services.

Parser failures must never silently fall through into an automatic legacy save
path. If an application retains an older importer, make fallback selection
explicit and observable.

## Choose a runtime boundary

| Boundary | Recommendation |
| --- | --- |
| Node.js or Next.js | Use `@fuzzy-parser/node` |
| Native Rust | Use `parser-api` |
| Scripts and contract debugging | Use `parser-cli` |
| Other languages | No supported network or native binding exists yet |

The Node package and Rust API expose reusable application profiles. The CLI
accepts raw schema JSON and is primarily an independent tool and verification
surface.

## Node WebAssembly library

`@fuzzy-parser/node` is the selected JavaScript boundary. It supports CommonJS
and ESM, calls the same Rust pipeline through WebAssembly, and runs each parse in
a Worker so deadlines or aborts can terminate and reap synchronous work.

The package is implemented and pack/install tested but not published. Until a
release is explicitly produced, consumers must use an authorized repository or
candidate artifact rather than assuming the npm name exists publicly.

```js
import {
  defineProfile,
  parseProfile,
  reviewRecords,
  unresolvedEvidence,
  ParserFailure,
  AdapterError,
} from "@fuzzy-parser/node";

const profile = await defineProfile({
  name: "contacts",
  version: "1",
  recordName: "contact",
  fields: [
    { name: "name", fieldType: "person_name", required: true },
    { name: "phone", fieldType: "phone_number" },
    { name: "amount", fieldType: "currency" },
    { name: "notes", fieldType: "text" },
  ],
});

try {
  const result = await parseProfile(
    profile,
    { format: "csv", bytes: uploadedBytes, filename: "contacts.csv" },
    {},
    { timeoutMs: 30_000, signal: abortController.signal },
  );

  const recordsToReview = reviewRecords(result);
  const unresolved = unresolvedEvidence(result);
} catch (error) {
  if (error instanceof ParserFailure) {
    // Safe structured parser report: error.report
  } else if (error instanceof AdapterError) {
    // Request, asset, timeout, abort, protocol, or output failure.
  }
}
```

The deadline covers the whole call, including Worker and runtime startup. A
separate `AbortSignal` can cancel an already-started call. Results are never
truncated.

The package validates generated JavaScript/WASM identity and parser/schema
versions before parsing. It has no CLI fallback, network service, or queue.

See the [package README](../packages/fuzzy-parser-node/README.md) for exact
runtime requirements and local package verification.

## Rust library

Use `parser-api` when the application runs in Rust:

```rust
use parser_api::{ApplicationInput, ApplicationProfile, ProfileField};
use parser_schema::FieldType;

let profile = ApplicationProfile::define("contacts", "1")
    .record_name("contact")
    .field(ProfileField::required("name", FieldType::PersonName))
    .field(ProfileField::optional("phone", FieldType::PhoneNumber))
    .field(ProfileField::optional("amount", FieldType::Currency))
    .field(ProfileField::optional("notes", FieldType::Text))
    .build()?;

let result = profile.parse(
    ApplicationInput::Csv {
        bytes: uploaded_bytes,
        file_name: Some("contacts.csv"),
    },
    Default::default(),
)?;
```

The API borrows caller bytes and does not open paths. Lower-level crates remain
available for callers that need direct schema, adapter, or plan control.

## Current CLI boundary

The CLI is useful for inspection, automation, and reproducing contracts:

```bash
cargo run -p parser-cli -- inspect fixtures/csv/comma.csv
cargo run -p parser-cli -- schema validate fixtures/schema/contact.json
cargo run -p parser-cli -- parse fixtures/csv/comma.csv \
  --schema fixtures/schema/contact.json
```

Successful data uses JSON stdout and exit `0`. Processing failures use JSON
stderr and exit `1`. Usage failures use plain stderr and exit `2`.

### CLI grammar and validation options

Run `cargo run -p parser-cli -- --help` for the authoritative grammar.

The CLI selects TXT, CSV, or XLSX by file extension. Standard input and inline
input are text. TXT files alone accept `--max-bytes` and
`--empty accept|reject`. Parse paths may accept table header, row, and XLSX
sheet selectors.

`--diagnostics` is accepted only before the command and may expose sensitive
context. Unknown, duplicate, misplaced, or extra arguments are rejected.

## Input and output validation

Applications should:

- pass an explicit input format rather than infer it from arbitrary content;
- treat the filename as optional display metadata, never an authorization path;
- enforce upload limits before invoking the parser;
- retain the original file when exact archival is required;
- accept compatible additive response fields;
- reject unsupported contract versions;
- keep raw source data out of ordinary logs;
- distinguish parser failures from adapter lifecycle failures.

Engine resource limits remain active even when an adapter has broader envelope
limits. Neither layer is a complete sandbox for hostile documents.

## Building a review UI

Do not reduce the response to a green/red confidence badge. A useful interface
shows:

- the original row or text segment;
- proposed fields in editable controls;
- concise review reasons;
- unresolved candidates and unused source;
- clear actions to cancel, copy, export, or explicitly continue.

Large imports should use a full-page or multi-step workspace rather than putting
the source, table, evidence, correction controls, and confirmation inside one
crowded modal.

See [Results and review](results-and-review.md) for the behavioral contract.

## Consumer-specific integration

QualEvents is the first consumer and validation case, not a dependency or source
of engine defaults. It supplies its own Contributor or Guest profiles and owns
Event authorization, duplicate and qualification rules, review/export, explicit
confirmation, persistence, and messaging.

The parser repository should contain only generic contracts and synthetic
examples. Detailed QualEvents migration, parity, and cutover planning belongs in
the QualEvents repository.

The same rule applies to every future consumer: transport can change, but parser
meaning remains caller-owned and the engine stays domain-neutral.

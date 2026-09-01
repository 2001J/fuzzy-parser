# Fuzzy Parser

[![CI](https://github.com/2001J/fuzzy-parser/actions/workflows/ci.yml/badge.svg?branch=development)](https://github.com/2001J/fuzzy-parser/actions/workflows/ci.yml?query=branch%3Adevelopment)

Fuzzy Parser turns inconsistent text, CSV, and XLSX input into structured records
that an application can review. It preserves the original source, explains its
assignments, and keeps uncertain or unused content visible.

```text
messy input + reusable application profile
    -> structured draft
    -> assigned fields + warnings + unresolved evidence
    -> application-owned review, export, or confirmation
```

The parser is domain-neutral. An application defines its field vocabulary once;
people uploading or pasting data do **not** write parser schemas for every import.
The application still owns permissions, duplicate rules, business validation,
corrections, persistence, and downstream side effects.

## What it does

- Reads pasted text, UTF-8 TXT, CSV, and XLSX input.
- Detects supported values such as names, text, phones, emails, numbers,
  currency, dates, booleans, and caller-defined enums.
- Assigns values using caller-provided field names, aliases, constraints, and
  table context.
- Returns source references, review reasons, warnings, and unused content.
- Produces deterministic drafts; it never treats a heuristic score as approval.

Fuzzy Parser does not save records, send messages, execute workbook formulas, or
invent application-specific rules.

## Quick start from source

The repository currently exposes Rust libraries, a CLI, and an installable but
not yet published Node/WebAssembly package.

```bash
cargo run -p parser-cli -- inspect fixtures/csv/comma.csv
cargo run -p parser-cli -- schema validate fixtures/schema/contact.json
cargo run -p parser-cli -- parse fixtures/csv/comma.csv \
  --schema fixtures/schema/contact.json
```

The CLI writes successful JSON to stdout. Processing failures are structured
JSON on stderr with exit code `1`; command-usage errors use exit code `2`.

For applications that repeatedly import the same kind of data, define a profile
once and reuse it:

```js
import {
  defineProfile,
  parseProfile,
  reviewRecords,
  unresolvedEvidence,
} from "@fuzzy-parser/node";

const contacts = await defineProfile({
  name: "contacts",
  version: "1",
  recordName: "contact",
  fields: [
    { name: "name", fieldType: "person_name", required: true, aliases: ["Full name"] },
    { name: "phone", fieldType: "phone_number", aliases: ["Mobile", "Telephone"] },
    { name: "amount", fieldType: "currency", aliases: ["Pledge", "Total"] },
    { name: "notes", fieldType: "text" },
  ],
});

const result = await parseProfile(contacts, {
  format: "csv",
  bytes: uploadedBytes,
  filename: "contacts.csv",
});

const recordsToReview = reviewRecords(result);
const unresolved = unresolvedEvidence(result);
```

Optional fields may be absent from an input without creating a new profile.
Change the profile version when field meaning, aliases, requiredness, or
constraints change.

## Choose an interface

| Interface | Use it for | Status |
| --- | --- | --- |
| `parser-api` Rust crate | Native applications and reusable typed profiles | Implemented in this workspace |
| `@fuzzy-parser/node` | Node.js and Next.js through Worker-isolated WebAssembly | Pack/install tested; not published |
| `parser-cli` | Inspection, automation, debugging, and contract verification | Implemented |
| HTTP service | Cross-language network integration | Not implemented |
| Graphical review application | Standalone human review | Not implemented |

There is one parser core behind these interfaces. The Node package is not a CLI
wrapper, queue, or separately operated service.

## Documentation

- [Getting started](docs/getting-started.md) — run the parser and understand the output.
- [Application profiles](docs/application-profiles.md) — define reusable fields and aliases.
- [Integration guide](docs/integration-strategy.md) — embed the parser safely.
- [Results and review](docs/results-and-review.md) — assignments, warnings, and source evidence.
- [Capability matrix](docs/current-state.md) — what is supported and what remains limited.
- [Documentation index](docs/README.md) — user, integrator, contributor, and maintainer paths.

Advanced wire-format details live in [data contracts](docs/data-contracts.md).
Implementation history, architecture decisions, audits, and runtime evaluations
are retained for maintainers but are not required reading.

## Development

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --workspace --locked
```

CI runs on pull requests and pushes to `development` and `main`. The manual
release workflow can build candidate artifacts without publishing them; public
release actions require an explicit protected invocation from `main`. See the
[contributor guide](docs/contributing.md) and [release guide](docs/releasing.md).

## License

Licensed under the [Apache License, Version 2.0](LICENSE).

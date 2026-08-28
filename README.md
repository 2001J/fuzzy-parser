# Fuzzy Parser

Fuzzy Parser turns messy text and tabular data into reviewable, traceable
records. It is a domain-neutral Rust engine: callers provide the fields,
aliases, enum values, and constraints; the parser provides extraction,
normalization, provenance, confidence, and uncertainty.

QualEvents is the first planned consumer and validation case, not an engine
dependency. Like any caller, it supplies input and its own schema/options; it
owns business rules, review/correction, export, and confirmed persistence.
Its adoption goal covers supported text and tabular imports. Independent Rust
library and CLI use remain part of the product.

The usable surface today is `parser-cli`: inspection of TXT, CSV, and XLSX,
schema validation, and partial schema-driven parsing. The QualEvents integration
is not implemented. See [current limitations](docs/current-state.md) and the
[engine roadmap](docs/roadmap.md).

## Quick Start

```bash
cargo run -p parser-cli -- inspect fixtures/text/simple.txt
cargo run -p parser-cli -- inspect fixtures/csv/comma.csv
cargo run -p parser-cli -- schema validate fixtures/schema/contact.json
```

Inspection preserves source locations and raw values. Valid output goes to
stdout; processing failures are structured JSON on stderr with exit code `1`.
Usage errors are plain text on stderr with exit code `2`.

## Documentation

Start with the [documentation guide](docs/README.md). It routes you by task:

- [Documentation index](docs/README.md)
- [Current state](docs/current-state.md)
- [Product direction](docs/product-direction.md)
- [Architecture](docs/architecture.md)
- [Parsing pipeline](docs/parsing-pipeline.md)
- [Data contracts](docs/data-contracts.md)
- [Error and confidence model](docs/error-and-confidence-model.md)
- [Testing strategy](docs/testing-strategy.md)
- [Continuous integration](docs/ci.md)
- [Roadmap](docs/roadmap.md)
- [Release and environment strategy](docs/release-and-environment-strategy.md)
- [Integration strategy](docs/integration-strategy.md)
- [Architecture decisions](docs/decisions/README.md)
- [Agent working rules](AGENTS.md)

## CLI Workflows

Inspect text or tables from a path, standard input, or inline content:

```bash
cargo run -p parser-cli -- inspect fixtures/text/simple.txt
cargo run -p parser-cli -- inspect fixtures/csv/comma.csv
cargo run -p parser-cli -- inspect fixtures/xlsx/sample.xlsx
printf 'Ada Lovelace\nGrace Hopper\n' | cargo run -p parser-cli -- inspect --stdin
cargo run -p parser-cli -- inspect --text $'Ada Lovelace\nGrace Hopper'
cargo run -p parser-cli -- schema validate fixtures/schema/contact.json
cat fixtures/schema/contact.json | cargo run -p parser-cli -- schema validate --stdin
cargo run -p parser-cli -- schema validate --text '{"schema_version":"0.1","record_name":"inline","fields":[],"options":{"allow_unknown_fields":true}}'
cargo run -p parser-cli -- schema validate --compact fixtures/schema/contact.json
```

Parse a CSV or text input against a caller schema, emitting a versioned parse result:

```bash
cargo run -p parser-cli -- parse fixtures/csv/comma.csv --schema fixtures/schema/contact.json
printf 'Ada Lovelace,ada@example.test\n' | cargo run -p parser-cli -- parse --stdin --schema fixtures/schema/contact.json
```

These examples assign **email only**: `contact.json` has no name field. The stdin
mode reads plain text, not CSV. The comma example assigns `ada@example.test`
at original UTF-8 bytes `13..29` (end exclusive), while `Ada Lovelace,` remains
unused source content. See [email boundary limits](docs/current-state.md#known-limitations).
Exit code `0` means processing succeeded; missing-field and ambiguity warnings
still require inspection. `source_evidence` embeds the unchanged canonical
document and accounts for unused, header and excluded content. Candidate source
references resolve to stored values; `parse.review` flags record-level reasons
for review. Neither `draft` nor `needs_review` means approval. See the
[source-coordinate and compatibility contract](docs/data-contracts.md#source-evidence-extension-and-compatibility).

Use `cargo run -p parser-cli -- --help` for command syntax. Schema validation
accepts a path, stdin, or inline text; `--compact <path>` emits one JSON line.
Validation accepts more field types than parsing: `text`, `person_name`, and
`datetime` are rejected by `parse` with `schema_field_type_unsupported`.
Parsing uses the same executable schema compiler available to Rust callers.
Only permissive `allow_unknown_fields=true` is supported; unknown schema members,
inapplicable constraints and unsupported enum definitions fail explicitly.
Enum ownership ties remain unassigned with warnings. See the
[executable schema capabilities and migration](docs/data-contracts.md#executable-schema).
See [integration usage](docs/integration-strategy.md) and
[the actual JSON contracts](docs/data-contracts.md).

## Container verification

CI [automated checks after code changes] builds and smoke-tests a non-root batch
CLI image without publishing it. It is not an HTTP service or a proven
QualEvents deployment boundary. See [CI checks and local reproduction](docs/ci.md)
and [release and publication rules](docs/release-and-environment-strategy.md).

## Development

Requires a stable Rust toolchain with edition 2024 support. The standard local
verification is:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --workspace --locked
```

The [workflow](.github/workflows/ci.yml) also checks Linux/macOS behavior, the
Node invocation prototype, WASM library compilation, dependency advisories,
and container semantics. Its tested Rust baseline is 1.96.0. CI and releases are
separate; no automatic publication is configured in this revision.

## License

Licensed under the [Apache License, Version 2.0](LICENSE).

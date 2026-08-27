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
printf 'Ada Lovelace ada@example.test\n' | cargo run -p parser-cli -- parse --stdin --schema fixtures/schema/contact.json
```

These examples assign **email only**: `contact.json` has no name field. The stdin
mode reads plain text, not CSV. A comma directly between a name and email can
currently prevent email detection ([regression #15](https://github.com/2001J/fuzzy-parser/issues/15)).
Exit code `0` means processing succeeded; missing-field and ambiguity warnings
still require inspection. The parse response does not yet include the complete
raw document or unused text; use `inspect` to examine extracted source values.

Use `cargo run -p parser-cli -- --help` for command syntax. Schema validation
accepts a path, stdin, or inline text; `--compact <path>` emits one JSON line.
Validation accepts more field types than parsing: `text`, `person_name`, and
`datetime` are rejected by `parse` with `schema_field_type_unsupported`.
See [integration usage](docs/integration-strategy.md) and
[the actual JSON contracts](docs/data-contracts.md).

## Container deployment

CI [automated checks after code changes] builds and smoke-tests a non-root batch
CLI image on pull requests and pushes to `main`; the latter also publishes
`ghcr.io/2001j/fuzzy-parser:latest`. It is not an HTTP service or a proven
QualEvents deployment boundary. See [release and publication rules](docs/release-and-environment-strategy.md).

## Development

Requires a stable Rust toolchain with edition 2024 support. The standard local
verification is:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace
```

GitHub Actions runs these checks and builds the CLI container. Container
publication is guarded to pushes on `main`.

## License

Licensed under the [Apache License, Version 2.0](LICENSE).

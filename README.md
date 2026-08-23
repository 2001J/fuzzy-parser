# Fuzzy Parser

Fuzzy Parser turns messy text and tabular data into reviewable, traceable
records. It is a domain-neutral Rust engine: callers provide the fields,
aliases, enum values, and constraints; the parser provides extraction,
normalization, provenance, confidence, and uncertainty.

The usable surface today is the `parser-cli` command. It reads TXT, CSV, and
XLSX input, validates caller-provided schemas, and emits JSON for scripts and
review tools.

## Quick Start

```bash
cargo run -p parser-cli -- inspect fixtures/text/simple.txt
cargo run -p parser-cli -- inspect fixtures/csv/comma.csv
cargo run -p parser-cli -- schema validate fixtures/schema/contact.json
```

Inspection preserves source locations and raw values. Valid output goes to
stdout; failures are structured JSON on stderr with a nonzero exit code.

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

Use `parser-cli --help` for all command modes. The schema validator accepts a path, standard input, or inline text. It emits pretty JSON by default, one compact JSON line with `--compact`, and structured errors on stderr.

## Container deployment

CI builds and smoke-tests the CLI image on every change and publishes `ghcr.io/<owner>/<repository>:latest` after pushes to `main`. Run it as a non-root batch container:

```bash
docker pull ghcr.io/<owner>/<repository>:latest
docker run --rm -v "$PWD:/workspace:ro" ghcr.io/<owner>/<repository>:latest inspect /workspace/fixtures/text/simple.txt
docker run --rm -i ghcr.io/<owner>/<repository>:latest schema validate --stdin < fixtures/schema/contact.json
```

This is a CLI deployment, not an HTTP service. Input files are mounted read-only and results are emitted as JSON on stdout or stderr.

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

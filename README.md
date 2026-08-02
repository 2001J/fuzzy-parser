# Fuzzy Parser

A domain-independent parsing engine written in Rust.

The project is structured as a Rust workspace containing the reusable parser core, input-format adapters, schema definitions, a command-line interface, and future TypeScript bindings.

## Project status

The project is currently under active development.

Initial work focuses on:

- Defining the canonical document model
- Reading plain-text input
- Providing structured errors
- Exposing functionality through a CLI
- Building a reliable automated test suite

Fuzzy field extraction, CSV support, spreadsheet support, and TypeScript bindings will be added incrementally.

See [Current State](docs/current-state.md) for the exact implemented behavior and [Roadmap](docs/roadmap.md) for planned releases.

## Requirements

Install the latest stable Rust toolchain using `rustup`.

Verify the installation:

```bash
rustc --version
cargo --version
```

## Workspace structure

```text
fuzzy-parser/
├── Cargo.toml
├── AGENTS.md
├── crates/
│   ├── parser-core/
│   ├── parser-formats/
│   ├── parser-schema/
│   └── parser-cli/
├── docs/
├── fixtures/
├── examples/
└── README.md
```

## Documentation

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

## Build

Build the entire workspace:

```bash
cargo build --workspace
```

## Test

Run all workspace tests:

```bash
cargo test --workspace
```

## Formatting

Check formatting:

```bash
cargo fmt --check
```

Automatically format the codebase:

```bash
cargo fmt
```

## Linting

Run Clippy and treat warnings as errors:

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

## Local checks

Run the following commands before opening a pull request:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace
```

## Running the CLI

The CLI currently supports raw TXT inspection:

```bash
cargo run -p parser-cli -- inspect fixtures/text/simple.txt
```

The command emits the canonical raw document as JSON. Its planned broader contract is documented in [Integration Strategy](docs/integration-strategy.md).

## Design principles

- The parser core must remain independent of any specific business domain.
- Consuming applications provide schemas and domain-specific rules.
- Original input must never be silently discarded or overwritten.
- Parsing errors and warnings must be structured and machine-readable.
- Every extracted value should remain traceable to its source.
- New functionality must include tests.

## License

Licensed under the [Apache License, Version 2.0](LICENSE).

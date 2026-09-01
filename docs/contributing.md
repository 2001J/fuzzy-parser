# Contributing

This page is the stable entry point for repository contributors. Product users
and application integrators can start at the [documentation index](README.md).

## Repository layout

| Crate | Responsibility |
| --- | --- |
| `parser-core` | Canonical models, detection, assignment, review, and provenance |
| `parser-formats` | Text, TXT, CSV, and XLSX input adapters |
| `parser-schema` | Schema models, validation, and compilation |
| `parser-api` | Application-facing reusable Rust profiles |
| `parser-cli` | Commands, streams, exit codes, and JSON presentation |

Read [Architecture](architecture.md) and [Parsing pipeline](parsing-pipeline.md)
before moving behavior across crate boundaries.

## Standard verification

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --workspace --locked
```

Use `tools/ci/verify-local.sh quick` or `full` when the ticket requires the
repository's broader deterministic checks. Node package work uses:

```bash
node tools/ci/verify-node-package.mjs
```

See [Testing strategy](testing-strategy.md) for risk-specific requirements and
[Continuous integration](ci.md) for hosted jobs.

## Branches and releases

- `development` is the long-lived integration branch.
- `main` is the stable branch.
- Feature work uses short-lived `codex/` branches.
- Publication requires an explicit protected release action from `main`.

See [Release and environment policy](release-and-environment-strategy.md) and
the [release operator guide](releasing.md).

## Historical evidence

Ticket audits, runtime experiments, and parallel-agent coordination are retained
under [Internal documentation](internal/README.md). They can explain why a
decision was made, but current behavior is established by code, executable
tests, and [Current state](current-state.md).

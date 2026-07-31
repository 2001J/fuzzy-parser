# 0001 — Use A Rust Workspace

## Status

Accepted.

## Context

The project needs a reusable parser engine, source-format adapters, schema definitions, a CLI, and future language bindings. Keeping all responsibilities in one crate would make boundaries unclear and encourage CLI or format-specific logic to leak into the core.

## Decision

Use a Rust workspace with initial crates:

- `parser-core`
- `parser-formats`
- `parser-schema`
- `parser-cli`

Keep dependency direction narrow and avoid circular dependencies.

## Consequences

- Components can be tested and packaged independently.
- The core can be reused by CLI, WebAssembly, native, or service interfaces.
- Cross-crate public models require deliberate ownership.
- Small early changes may touch more than one manifest, but the separation prevents a later monolith split.

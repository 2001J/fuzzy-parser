# Architecture Decisions

This directory contains ADRs [architecture decision records] for choices that are important, durable, or expensive to reverse.

## Status values

- `Proposed`: under discussion.
- `Accepted`: current decision.
- `Superseded`: replaced by another ADR.
- `Rejected`: considered and not adopted.

## Index

- [0001 — Use a Rust workspace](0001-use-rust-workspace.md)
- [0002 — Use schema-driven domain-neutral parsing](0002-schema-driven-domain-neutral-parsing.md)
- [0003 — Preserve raw input and provenance](0003-preserve-raw-input-and-provenance.md)
- [0004 — Deliver the CLI before language bindings](0004-cli-before-language-bindings.md)

## ADR template

```md
# NNNN — Decision title

## Status

Proposed | Accepted | Superseded | Rejected

## Context

What problem or pressure required a decision?

## Decision

What was chosen?

## Consequences

What becomes easier, harder, required, or forbidden?
```

Create a new ADR when changing crate boundaries, public contracts, parser determinism, schema ownership, provenance guarantees, integration order, or release packaging.

# 0002 — Use Schema-Driven Domain-Neutral Parsing

## Status

Accepted.

## Context

The parser should work as an independent product and also integrate into applications such as QualEvents. Hardcoding guest, pledge, payment, or invitation concepts would make the Rust engine dependent on one product and prevent reuse.

A completely assumption-free parser cannot know whether an unlabelled number is a capacity, amount, identifier, or line number. The caller must therefore describe the structure it wants.

## Decision

The parser receives caller-provided schemas and options that define generic fields, aliases, locale hints, and constraints.

Business applications own their domain profiles and workflows. The parser core owns generic extraction, candidate detection, assignment, uncertainty, and provenance.

## Consequences

- The same engine can serve contacts, transactions, inventory, event data, and other domains.
- Consuming applications must define schemas or select profiles.
- Domain-specific validation remains outside the core unless expressed through generic caller-provided constraints.
- Parser output may remain ambiguous when the schema and input do not provide enough evidence.

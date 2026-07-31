# Fuzzy Parser Documentation

This directory contains the project documentation that is too detailed or too changeable for the root README.

## Start here

- [Current state](current-state.md) — what exists in the repository today.
- [Product direction](product-direction.md) — what problem the project solves and what it intentionally does not own.
- [Architecture](architecture.md) — crate responsibilities, dependency direction, and deployment shapes.
- [Parsing pipeline](parsing-pipeline.md) — the stage-by-stage processing model and invariants.
- [Data contracts](data-contracts.md) — canonical input, schema, candidate, and output models.
- [Error and confidence model](error-and-confidence-model.md) — fatal failures, warnings, ambiguity, provenance, and confidence.
- [Testing strategy](testing-strategy.md) — unit, integration, fixture, regression, fuzz, and benchmark expectations.
- [Roadmap](roadmap.md) — the intended order of incremental releases.
- [Release and environment strategy](release-and-environment-strategy.md) — branch, environment, packaging, and publication rules.
- [Integration strategy](integration-strategy.md) — CLI-first development and future TypeScript/WebAssembly/service integration.
- [Architecture decisions](decisions/README.md) — durable records of important technical choices.

## Documentation rules

Each document has one primary responsibility:

- `current-state.md` must remain factual. Planned work belongs elsewhere.
- `product-direction.md` explains user value and project boundaries, not implementation detail.
- `architecture.md` owns component and crate boundaries.
- `parsing-pipeline.md` owns stage behavior and invariants.
- `data-contracts.md` owns public model shapes and serialization expectations.
- `roadmap.md` may change frequently and must not be treated as implemented behavior.
- Architecture decisions should be captured as ADRs [architecture decision records] when reversing the decision later would be expensive or confusing.

When behavior changes, update the narrowest authoritative document instead of copying the same explanation into several files.

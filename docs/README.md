# Fuzzy Parser Documentation

This is the project map. Start with the question you need answered.

## Find Your Way

| Question | Document |
| --- | --- |
| What can I use today? | [Current state](current-state.md) |
| How do I run or deploy it? | [README](../README.md) and [release strategy](release-and-environment-strategy.md) |
| What problem does it solve? | [Product direction](product-direction.md) |
| How does input become structured output? | [Parsing pipeline](parsing-pipeline.md) |
| What models and JSON contracts exist? | [Data contracts](data-contracts.md) |
| How are errors and uncertainty represented? | [Error and confidence model](error-and-confidence-model.md) |
| Which crate owns a behavior? | [Architecture](architecture.md) |
| How should changes be tested? | [Testing strategy](testing-strategy.md) |
| Which automated checks run, and how do I reproduce them? | [Continuous integration](ci.md) |
| What is planned next? | [Roadmap](roadmap.md) |
| Who is working on which ticket, and how are branches combined? | [Parallel work board](parallel-work.md) |
| How are releases and containers handled? | [Release and environment strategy](release-and-environment-strategy.md) |
| How will other runtimes integrate? | [Integration strategy](integration-strategy.md) |
| Why was a design decision made? | [Architecture decisions](decisions/README.md) |
| Why did the old backlog statuses change? | [2026-08-27 acceptance audit](audits/2026-08-27-backlog.md) |

## Documentation rules

Each document has one primary responsibility:

- `current-state.md` must remain factual. Planned work belongs elsewhere.
- `product-direction.md` explains user value and project boundaries, not implementation detail.
- `architecture.md` owns component and crate boundaries.
- `parsing-pipeline.md` owns stage behavior and invariants.
- `data-contracts.md` owns current serialized shapes and clearly marked proposed models.
- `roadmap.md` may change frequently and must not be treated as implemented behavior.
- `parallel-work.md` is the coordinator-owned assignment and integration board, not a second product roadmap or proof of a release.
- `integration-strategy.md` owns the reusable boundary and separately owned
  first-consumer handoff; it does not certify deployment or host adoption.
- `error-and-confidence-model.md` owns diagnostics and heuristic-score semantics;
  `testing-strategy.md` owns verification requirements.
- `release-and-environment-strategy.md` owns versioning and publication rules.
  Planning milestones are not package or contract versions.
- `ci.md` owns workflow gates, local reproduction and hosted-verification limits;
  CI is not release automation or evidence of host adoption.
- Dated audits record historical evidence, not a competing current-state contract.
- Architecture decisions should be captured as ADRs [architecture decision records] when reversing the decision later would be expensive or confusing.

When behavior changes, update the narrowest authoritative document instead of copying the same explanation into several files.
Use **QualEvents** consistently for the first consumer, **Fuzzy Parser** for the
engine, and **schema/profile** for caller-owned structure and policy. Distinguish
implemented behavior, the approved integration plan, and later possibilities.

# 0005 — Validate An Independent Engine Through Its First Consumer

## Status

Accepted planning direction, 2026-08-27. No new runtime boundary or independence
test result is claimed by this decision. Partially supersedes the process-next
sequence in [ADR 0004](0004-cli-before-language-bindings.md); its CLI-first
requirement remains accepted and fulfilled.

## Context

The CLI now inspects TXT/CSV/XLSX and performs partial schema-driven parsing.
The previous roadmap implied a standalone UI, process integration, WASM and
other surfaces in sequence, while old tracker labels still described a TXT-only
release. QualEvents provides a concrete first-consumer validation case, but
consumer adoption must not become the definition of generic engine completion.

An initial planning draft named milestones after QualEvents and included its
full migration as an engine epic. That framing is superseded here: host review,
UI, migration and cutover are external responsibilities. Useful generic format,
profile and source-evidence gaps remain in the engine backlog.

## Decision

- Prioritize an independent, reviewable text/tabular engine. Accept raw input and
  caller-owned schema/options; return generic records, source evidence, warnings,
  unused/rejected content and unresolved values.
- Keep independent library/CLI use. No consumer-specific constants, schemas,
  identifiers, imports or dependencies belong in generic engine behavior.
  Synthetic consumer-shaped fixtures must be isolated from implementation.
- Select one reusable runtime boundary using real deployment constraints,
  including those of the first consumer. This ADR delegated selection to
  [#11](https://github.com/2001J/fuzzy-parser/issues/11); the reviewed outcome is
  now recorded in [ADR 0006](0006-library-interface-runtime-evaluation.md), and
  other transports are not prerequisites.
- Require a future independence gate: the same unmodified engine/public
  interface processes a synthetic QualEvents-shaped profile and an unrelated
  supported-domain profile using caller configuration only, with QualEvents
  not installed or available. [#19](https://github.com/2001J/fuzzy-parser/issues/19)
  owns that planned conformance test. Do not claim unsupported fields work today.
- Keep generic engine readiness/coverage separate from external host adoption.
  QualEvents owns Event/Guest/Contributor concepts, business validation,
  authorization, duplicate and qualification policy, review/correction UI,
  export workflow, confirmed persistence and messaging effects.
- Preserve the intended adoption goal: Fuzzy Parser should eventually power all
  supported QualEvents text/tabular import processing. The host must preserve
  working paths until its own parity and rollback gates pass. That migration is
  not an engine implementation dependency or closure criterion.

## Consequences

The [roadmap](../roadmap.md) uses generic named milestones distinct from software
versions. [Integration strategy](../integration-strategy.md) owns the reusable
boundary and separate host handoff; it is not a claim of host readiness.
Source preservation and conformance precede further heuristics. Other consumers,
standalone review tooling, additional transports and PDF/OCR remain later work.
No release, deployment or host modification is authorized by this decision.

# 0003 — Preserve Raw Input And Provenance

## Status

Accepted.

## Context

Messy input is ambiguous. Normalization and heuristics can make mistakes, and users need to understand or correct parser output. If raw values or source locations are overwritten, review becomes difficult and silent data loss becomes possible.

## Decision

Preserve original source values and locations throughout the pipeline.

Normalized values, record candidates, field candidates, and assignments must reference the source blocks or spans from which they were derived. Rejected and unused fragments must remain observable.

## Consequences

- Review interfaces can show parser suggestions beside original evidence.
- Regression debugging is easier.
- Models and responses carry more metadata.
- Privacy-sensitive integrations may need output modes that omit raw content while retaining opaque source references.
- Destructive normalization without a recorded transformation is forbidden.

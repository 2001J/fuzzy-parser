# Cross-profile Conformance

The #19 independence gate is implemented with synthetic caller-owned profiles
and no QualEvents code, package, runtime, or dependency. One attendance-shaped
profile and one unrelated inventory profile run through the same unmodified
Rust library, CLI, CJS package entry, ESM package entry, Worker protocol, and
WASM artifact.

| Capability | Permanent evidence |
| --- | --- |
| Two profiles across pasted text, TXT, CSV, and XLSX | `crates/parser-cli/tests/parse/conformance.rs` compares native/CLI complete output; `packages/fuzzy-parser-node/test/conformance.test.mjs` repeats the corpus through CJS and ESM |
| Clean plus ambiguous records | Shared fixtures require exact assignments, `needs_review`, required/ambiguity warnings, unassigned candidates, and nonempty unused source |
| Determinism and provenance | Repeated byte-identical JSON plus resolution of detected, assigned, and unassigned source references |
| Consumer independence | Tests scan engine/package implementation for the synthetic record names and `QualEvents`; schemas remain fixture data only |
| Unicode, typed/blank XLSX, and byte input | Package format tests plus the existing #21 and #22 regressions |
| Comma-email and multiline offsets | Dedicated #15 core/CLI regressions remain in the same full suite |
| Missing/unsupported fields and malformed input | Shared schema/error suites and package `ParserFailure` tests retain safe typed failures |
| Header ambiguity and table selection | #16 core/schema/CLI regressions and package table-selection provenance test |
| Resource limits | #17 exact/one-over crate/CLI coverage plus package schema/result/message boundary tests |

This gate proves engine/interface independence for the implemented capability
set. It does not certify parsing accuracy, business approval, a QualEvents
review flow, persistence, export, messaging, deployment, publication, browser
support, Vercel behavior, or production capacity.

Generic gaps remain tracked in [#20](https://github.com/2001J/fuzzy-parser/issues/20),
including legacy/extended formats and broader field/locale capabilities. Current
known limits such as unsupported `datetime`, legacy `.xls`, PDF/OCR, formula
semantics, and uncalibrated confidence remain explicit in
[current state](current-state.md).

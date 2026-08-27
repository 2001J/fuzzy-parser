# Roadmap

This document owns development order. [Current state](current-state.md) owns
implemented capabilities; [integration strategy](integration-strategy.md) owns
the consumer boundary and rollout. Milestones below are plans, not releases.

## First direction: independently reviewable imports

Prioritize generic pasted/TXT and tabular parsing, review evidence, and one
reusable runtime boundary. QualEvents is the first real consumer and validation
case, not the owner of the engine contract or a build/runtime dependency.
Independent Rust library and CLI operation remain supported.

Its eventual adoption should cover all its supported text/tabular imports, not
just optional pasted-text help. Host review, UI, migration and cutover are
separate work described in [integration strategy](integration-strategy.md);
they neither block nor establish generic engine readiness.

[#10 — Preserve source evidence and unused content in the versioned parse
response](https://github.com/2001J/fuzzy-parser/issues/10) is implemented and
independently verified; [data contracts](data-contracts.md) describes the
canonical source-review extension. The next focused slice is
[#21 — Prevent Unicode label-context slicing from crashing assignment](https://github.com/2001J/fuzzy-parser/issues/21),
a pre-existing bug found during that review. Evaluate the runtime boundary
early in [#11](https://github.com/2001J/fuzzy-parser/issues/11). These individual
steps do not establish complete engine readiness or authorize publication.

## Milestone: Reviewable import engine

[GitHub milestone](https://github.com/2001J/fuzzy-parser/milestone/1) ·
[tracking epic #9](https://github.com/2001J/fuzzy-parser/issues/9)

Every implementation ticket has explicit tests and dependencies. A dependency
must be satisfied before the dependent ticket completes; ready work can be
selected independently.

| Work | Dependency / gate |
| --- | --- |
| [#10 Source-complete result and review reasons](https://github.com/2001J/fuzzy-parser/issues/10) | Implemented and independently verified; includes retained raw-model compatibility tests |
| [#21 Unicode-safe assignment context](https://github.com/2001J/fuzzy-parser/issues/21) | Ready next; fix the pre-existing panic with permanent core/CLI regressions |
| [#11 Select one reusable runtime boundary](https://github.com/2001J/fuzzy-parser/issues/11) | Early evaluation; prove the chosen target, not all alternatives |
| [#2 Finish safe structured errors](https://github.com/2001J/fuzzy-parser/issues/2) | Ready; path redaction and error coverage remain |
| [#4 Permanent TXT adapter edge-case fixtures](https://github.com/2001J/fuzzy-parser/issues/4) | Ready; adapter exists, durable acceptance coverage is incomplete |
| [#5 Reusable file validation and empty policy](https://github.com/2001J/fuzzy-parser/issues/5) | #2 |
| [#6 Strict CLI dispatch and arguments](https://github.com/2001J/fuzzy-parser/issues/6) | #2, #5 |
| [#7 Complete TXT subprocess matrix](https://github.com/2001J/fuzzy-parser/issues/7) | #2, #4, #5, #6 |
| [#12 Shared schema compilation/capability validation](https://github.com/2001J/fuzzy-parser/issues/12) | #2; prevents separate parsing logic per interface |
| [#13 Caller-directed text and name fields](https://github.com/2001J/fuzzy-parser/issues/13) | #10, #12 |
| [#14 Compose text normalization/segmentation](https://github.com/2001J/fuzzy-parser/issues/14) | #10, #12 |
| [#15 Delimiter-adjacent email regression](https://github.com/2001J/fuzzy-parser/issues/15) | Ready; preserve original byte offsets |
| [#16 Explicit table headers/selection/provenance](https://github.com/2001J/fuzzy-parser/issues/16) | #10, #12; coordinate bounds with #17 |
| [#17 Bound CSV/XLSX/schema/result resource use](https://github.com/2001J/fuzzy-parser/issues/17) | #2, #5, #12 |
| [#18 Implement the selected runtime adapter](https://github.com/2001J/fuzzy-parser/issues/18) | #10, #11, #12, #17; final parity includes #13–#16 |
| [#19 Cross-profile conformance and independence](https://github.com/2001J/fuzzy-parser/issues/19) | All preceding engine-readiness work |

The milestone ends with a tested independent engine and reusable boundary.
It includes the [planned cross-profile independence gate](testing-strategy.md#cross-profile-conformance-and-independence--planned)
in #19, which has not been verified today. It does **not** assert that any host
review flow, production deployment, or migration has shipped.

## Milestone: Extended format and profile coverage

[GitHub milestone](https://github.com/2001J/fuzzy-parser/milestone/2) ·
[tracking epic #20](https://github.com/2001J/fuzzy-parser/issues/20)

Extend the generic capability matrix: legacy XLS, declared TSV/delimited TXT,
display/date/number handling, sheet/style metadata, and caller-supplied fields
and interpretation options. Split concrete implementation children with tests
before execution. Added capabilities must retain the cross-profile independence
gate. No Event/Guest/Contributor types or consumer-specific schemas/constants
belong in the engine.

This milestone does not track legacy-path retirement or QualEvents cutover.
Those remain external host work, informed by the engine capability matrix.
Working host imports must remain available until their replacement passes
host-owned parity tests; engine completion alone does not authorize migration.

## Later work

- Additional consumers and reusable profiles, justified by actual integration needs.
- Standalone schema editor/review/export tool using the same engine.
- Other runtime surfaces only when a measured deployment need justifies them.
- More generic datetime/locale/assignment capabilities outside the readiness slice.
- Broader property tests, fuzzing, and measured benchmarks; minimum input safety
  and regressions are engine-readiness requirements, not postponed here.
- Text-based PDF, then OCR; neither precedes reliable deterministic text/table review.
- Optional correction-learning research with explicit privacy design.

## Version and history reconciliation

The workspace/package version is `0.1.0`; parse and schema contracts each use
`0.1`. These are independent version axes, governed by
[release strategy](release-and-environment-strategy.md). No version bump, tag, or
release is authorized by a milestone name.

The old roadmap's `0.1`–`0.14` headings were planning stages, not shipped package
versions. The old [TXT-only v0.1 epic #8](https://github.com/2001J/fuzzy-parser/issues/8)
used a conflicting meaning. It is superseded as a plan, **not completed as an
acceptance gate**. Its unfinished criteria survive in #2 and #4–#7.

| Former stage | Reconciled status / destination |
| --- | --- |
| 0.1 Workspace foundation | Implemented workspace and automated checks |
| 0.2 TXT inspection | Working path; validation/privacy/test gaps remain in #2, #4–#7 |
| 0.3 Pasted text/dispatch | Text/stdin exist; strict file dispatch remains #5/#6 |
| 0.4 CSV / 0.5 XLSX | Adapters exist; table compatibility and limits remain #16/#17/#20 |
| 0.6 Normalization / 0.7 Segmentation | Separate library stages exist; document composition remains #14 |
| 0.8 Schema | Model/validation exist; shared executable capabilities remain #12/#13 |
| 0.9 Detection / 0.10 Assignment | Partial implementation; gaps go to #12–#16 and later coverage |
| 0.11 Explainable result | Canonical source/review extension implemented and independently verified in #10; broader engine-readiness gates remain open |
| 0.12 Standalone / 0.13 WASM | Later possibilities; #11 selects the reusable boundary |
| 0.14 Reliability | Required safety/regressions move into readiness tickets; broad fuzzing/benchmarks follow |

[The dated acceptance audit](audits/2026-08-27-backlog.md) records the code,
tests, manual probes, and issue dispositions used for this reconciliation.

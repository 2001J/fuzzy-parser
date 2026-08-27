# Integration Strategy

This document owns the reusable integration boundary and external consumer
handoff. Use
[data contracts](data-contracts.md) for exact current versus proposed models,
[current state](current-state.md) for capability limits, and [roadmap](roadmap.md)
for ticket order. QualEvents integration is planned, not implemented.

## Generic engine/adapter/host ownership

```text
Any caller's uploaded/pasted input + caller-owned schema/options
    → Fuzzy Parser format extraction and canonical source document
    → generic normalization, segmentation, detection, assignment
    → structured draft with source evidence, uncertainty, warnings, unused content
    → caller-owned review and correction
    → caller-owned export OR explicit confirmation and persistence
```

Fuzzy Parser is an independent engine. A reusable adapter changes transport and
runtime packaging, not domain meaning or parsing behavior. QualEvents is the
first consumer and one validation case; it does not define public types,
defaults, identifiers, package dependencies, or engine completion.

| Fuzzy Parser engine / reusable adapter | Host application (QualEvents examples) |
| --- | --- |
| Generic source adapters, raw values and provenance | Upload/paste controls and schema/profile selection |
| Normalization, record segmentation, candidate detection/assignment | Event scope, permissions, duplicate policy, qualification |
| Generic constraints, uncertainty, warnings and unused/rejected content | Review/correction, export, explicit confirmation and database writes |
| One reusable core and versioned runtime boundary | Messaging and other downstream business effects |

Parsing must not cause business effects. The host revalidates corrected values
and authorization at confirmation, even when a parser suggestion has a high score.
No consumer-specific constants, schemas, identifiers, imports or dependencies
belong in generic engine behavior. Synthetic examples must be isolated fixtures.

### Planned independence acceptance gate

The reusable boundary must pass [#19](https://github.com/2001J/fuzzy-parser/issues/19)
and the [cross-profile independence gate](testing-strategy.md#cross-profile-conformance-and-independence--planned)
defined in testing strategy. That gate is **planned, not verified**; unsupported
field types and interfaces must remain explicit until implemented.

## Current CLI boundary

The CLI is implemented and remains the independent verification surface.
Run from the repository root:

```bash
cargo run -p parser-cli -- inspect fixtures/text/simple.txt
cargo run -p parser-cli -- inspect fixtures/csv/comma.csv
cargo run -p parser-cli -- inspect fixtures/xlsx/sample.xlsx
cargo run -p parser-cli -- schema validate fixtures/schema/contact.json
cargo run -p parser-cli -- parse fixtures/csv/comma.csv --schema fixtures/schema/contact.json
printf 'Ada Lovelace ada@example.test\n' | cargo run -p parser-cli -- parse --stdin --schema fixtures/schema/contact.json
```

`parse` takes a positional path, not `--input`. Its stdin mode is plain text;
there is no JSON request envelope or `parse --text` mode. The sample schema
requests only email: expect two CSV data records with emails and one stdin
record with `ada@example.test`. Names are not assigned. A comma adjoining the
email currently breaks the text example; [#15](https://github.com/2001J/fuzzy-parser/issues/15)
tracks the regression.

Successful results use JSON stdout and exit `0`, including records with warnings.
Processing errors use JSON stderr and exit `1`; usage errors use plain stderr
and exit `2`. `cargo run` itself may print build diagnostics to stderr; use the
built `target/debug/parser-cli` when asserting the binary's streams.

`ParseResponse` now includes canonical source evidence, unused content and
draft/review reasons. Its [additive compatibility contract](data-contracts.md#source-evidence-extension-and-compatibility)
distinguishes cell/string coordinates from original file bytes and legacy
responses without evidence. This does not establish complete integration
readiness: schema compilation still lives in the CLI ([#12](https://github.com/2001J/fuzzy-parser/issues/12)),
several field types are unsupported, and runtime/independence gates remain open.

## Proposed library caller experience

The intended integration is an independently reusable library with a small
generic call. No message queue or separately operated parser service is needed
or authorized for the initial integration. A bundled executable behind a JS
wrapper would still run locally with the caller, but the package must handle
native binaries, process lifecycle and temporary files. A WASM module could
avoid that plumbing; its actual byte-input and runtime gates remain open.

**Proposal only — not an existing npm API or executable example.** Exact types
depend on #12/#18 and must reuse [data contracts](data-contracts.md), not create
a second schema model:

```typescript
const draft = await parser.parse({
  input: { format: "csv", bytes: uploadedBytes },
  schema: callerSchema,
  options: callerOptions,
});
```

The caller supplies the input and its own schema/options, then receives
source-backed draft records, warnings and unresolved content. It owns mapping,
review/correction, export and confirmed persistence; core fixes belong here.
The interface must also work with unrelated supported profiles and no QualEvents.

## Runtime evaluation — backend decision still open

[ADR 0005](decisions/0005-independent-engine-consumer-validation.md) retains CLI-first but
supersedes the unconditional process-next sequence in ADR 0004.
The bounded [#11](https://github.com/2001J/fuzzy-parser/issues/11) evaluation
retains a successful Node/CLI prototype and a WASM compilation/source check.
[ADR 0006](decisions/0006-library-interface-runtime-evaluation.md) owns the
comparison, proposed library boundary and outstanding backend gate. The bounded
evidence has passed independent review. [Dated evidence](evaluations/2026-08-28-node-cli.md) owns reproducible
commands and measured results. No production adapter or Vercel deployment is
claimed. [#18](https://github.com/2001J/fuzzy-parser/issues/18) implements only
the reviewed choice after its engine prerequisites.

The prototype invokes the current CLI without copying schema conversion.
Its two fixture profiles demonstrate only supported integer/boolean subsets,
not the full #19 independence gate. WASM is a credible candidate, not rejected
because today's CLI was easier to exercise. After shared schema compilation
and [#22's XLSX byte-reader gap](https://github.com/2001J/fuzzy-parser/issues/22) are addressed, #11 needs a separately
authorized bounded JS/WASM comparison before selecting one backend for #18.
No second adapter is built in this slice. Native bindings are deferred; queues
and a separate service are outside the initial direction.
The first consumer's Node.js/Next.js configuration informs
the evaluation; its deployed version, architecture and compute settings remain
unverified. Host installation/migration does not gate generic engine readiness.

The current container is a batch CLI, not an HTTP API. If the chosen boundary
needs a separately authorized nonproduction deployment test, keep that gate open.
All runtime surfaces must call the same library schema compilation and parser
pipeline. The adapter must accept arbitrary supported caller schemas/options,
run with QualEvents absent, and introduce no host dependency. Engine readiness
is separate from whether a particular host has installed or adopted the adapter.

## Grounding in existing QualEvents imports

Read-only review on 2026-08-27 used QualEvents commit
`50fcaf072abd5307157ce1e0ee96676729e896c5`, not a production-runtime check:

- [Contribution import page](https://github.com/2001J/digital-invitation/blob/50fcaf072abd5307157ce1e0ee96676729e896c5/app/admin/contribution/import/page.tsx)
  uploads files and offers clean-sheet preparation.
- [Contribution import route](https://github.com/2001J/digital-invitation/blob/50fcaf072abd5307157ce1e0ee96676729e896c5/app/api/contribution/contributors/import/route.ts)
  authenticates and dispatches preparation or import; it is not yet a generic
  Fuzzy Parser preview/confirmation API.
- [Contribution import helper](https://github.com/2001J/digital-invitation/blob/50fcaf072abd5307157ce1e0ee96676729e896c5/lib/contribution-contributor-import.ts)
  accepts CSV/TSV/delimited TXT/XLSX/XLS and owns domain validation, duplicates,
  payment handling and promotion decisions.
- [Table helper](https://github.com/2001J/digital-invitation/blob/50fcaf072abd5307157ce1e0ee96676729e896c5/lib/import-table-parser.ts)
  searches for known headers, prefers an upload-ready sheet, and can select
  highlighted rows. These behaviors are not equivalent to today's Rust adapters.
- Guest paths are separate:
  [upload](https://github.com/2001J/digital-invitation/blob/50fcaf072abd5307157ce1e0ee96676729e896c5/app/api/guests/upload/route.ts)
  and [prepare-upload](https://github.com/2001J/digital-invitation/blob/50fcaf072abd5307157ce1e0ee96676729e896c5/app/api/guests/prepare-upload/route.ts).

The existing host issues [#57](https://github.com/2001J/digital-invitation/issues/57),
[#59](https://github.com/2001J/digital-invitation/issues/59), and
[#19](https://github.com/2001J/digital-invitation/issues/19) document Event and
lifecycle guarantees to preserve. They are not Fuzzy Parser integration tickets.
This pass does not modify the host repository or tracker.
The [2026-08-28 runtime evaluation](evaluations/2026-08-28-node-cli.md#read-only-consumer-evidence)
revalidates route/runtime configuration at the same commit without running the host.

## Future QualEvents work: separately owned

These are host planning slices, not Fuzzy Parser implementation tickets. Create
or refine host tickets only in a separately authorized QualEvents task.

The host adoption objective remains broad: Fuzzy Parser should eventually power
all supported QualEvents text/tabular imports, not just optional pasted-text
assistance. This objective is recorded here as a consumer handoff; its UI,
legacy-route migration and cutover are not Fuzzy Parser implementation tickets
or conditions for generic engine readiness.

| Slice | Depends on | Required host verification |
| --- | --- | --- |
| Profiles and thin bridge | Selected boundary, #10/#12/#18 contracts | Caller field/alias/custom-field/locale behavior; contract mismatch and unsupported capabilities fail explicitly |
| Review/correction and source display | Engine conformance #19 and bridge | Paste/TXT/CSV/XLSX drafts, source references, warning/unused-content visibility, edits and rejection; no writes/messages from parsing or preview |
| Export and explicit confirmation | Reviewed records and existing domain services | Clean-sheet/export behavior, formula-safe exports, server revalidation, auth/Event scope, duplicates and qualification; persistence only for explicitly confirmed valid rows |
| Staged Contributor then Guest cutover | Host parity matrix informed by generic capabilities in #20 | Existing accepted files/profiles and route behaviors remain available; separate Guest guarantees, explicit routing, rollback, no silent fallback on engine failure |

Use a feature gate [a switch controlling which implementation a flow uses] and
synthetic comparison fixtures before changing the default. Unsupported legacy
XLS, formatting/style selection, or profile rules remain on an explicit legacy
path until equivalent engine support and host parity are verified. Record host
exceptions/removal gates in future host work. Generic capability gaps feed
[#20](https://github.com/2001J/fuzzy-parser/issues/20), which does not track or
block on the host's migration.
Do not leave an undocumented permanent competing parser, narrow the overall
engine role, or treat numeric confidence as approval.

## Later consumers and tooling

Standalone review/export tooling, other products, additional bindings, and
PDF/OCR remain in the [roadmap](roadmap.md). They reuse the same core and must
not delay the first tested reusable boundary. Rust models remain authoritative;
any TypeScript types must be generated or checked for contract parity.

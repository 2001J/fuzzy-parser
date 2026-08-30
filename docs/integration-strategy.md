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
record with `ada@example.test`. Names are not assigned. Comma-adjacent email
is supported by the reviewed [#15 fix](https://github.com/2001J/fuzzy-parser/issues/15),
with original byte spans and unused prefix preserved. This remains a limited
ASCII email detector, not comprehensive email syntax validation.

Successful results use JSON stdout and exit `0`, including records with warnings.
Processing errors use JSON stderr and exit `1`; usage errors use plain stderr
and exit `2`. Help is the explicit plain-text stdout/exit `0` exception.
The opposite stream is empty in each case. `cargo run` itself may print build
diagnostics to stderr; use the built `target/debug/parser-cli` when asserting
the binary's streams.

The [error contract 0.1 migration](data-contracts.md#error-contract-01-and-migration-from-unversioned-errors)
changes default error fields and messages, not successful results. Default errors
omit paths and caller values. For authorized troubleshooting only, place
`--diagnostics` **before** the command:

```bash
cargo run -p parser-cli -- inspect /synthetic/private/missing.txt
cargo run -p parser-cli -- --diagnostics inspect /synthetic/private/missing.txt
```

Both commands intentionally exit `1` with `io_error` / `not_found`. The first
has no path; the second adds `error.diagnostics.path` and escaped context in the
outer message. Diagnostics may contain private data; do not use them in public
logs. There is no environment opt-in, and input/schema content, filenames or
trailing arguments containing `--diagnostics` do not enable it. Bare flag-like
paths and extra tails now fail usage validation; prefixed paths and inline
content remain data. Only one leading diagnostic switch is accepted.
Rust callers opt in with `ParserError::report(DiagnosticsMode::Detailed)` or the
equivalent schema/shared-failure report method. Default `Display` remains safe.

### CLI grammar and validation options

Every invocation accepts one optional leading `--diagnostics`, followed by
exactly one of these forms:

```text
inspect <path> [TXT_OPTIONS]
inspect --stdin
inspect --text <content>
parse <path> --schema <schema-path> [PATH_OPTIONS]
parse --stdin --schema <schema-path>
schema validate <path>
schema validate --stdin
schema validate --text <content>
schema validate --compact <path>
```

`-h` or `--help` alone works at root, `inspect`, `parse`, `schema`, and
`schema validate`, also after leading diagnostics. Help with extra tokens is a
usage error. There is no `--input`, `parse --text`, `--` terminator or compact
stdin/text mode. Bare path tokens beginning `-` are usage errors; use `./-name`
or an absolute path. Paths retain native OS encoding. `--text` consumes its
next token literally, including `--help` or `--diagnostics`.

`TXT_OPTIONS` contains `--max-bytes N`, `--empty accept|reject`, or both, in either
order, at most once each, after the input path (after the schema path for
`parse`). `N` is nonempty ASCII decimal digits fitting the platform's `usize`;
zero and leading zeros are allowed. Signs, spaces, units, equals syntax,
unknown flags, duplicates and extra/misplaced/missing tokens are usage errors.
Usage prints only `usage: parser-cli --help`, with the retained exception
`text argument must be valid UTF-8` for non-UTF-8 `inspect --text` content.
Non-UTF-8 `schema validate --text` retains `schema_input_error`/exit `1`.
Malformed syntax takes priority over content encoding and processing errors.

Overrides apply only to TXT file input in `inspect`/`parse`, never schema files,
stdin/inline text, CSV or XLSX. Passing them to known CSV/XLSX inputs is usage/`2`,
even when equal to defaults. Well-formed options on an unknown extension still
produce `unsupported_input`/`1`; malformed values or arity produce usage/`2` first.
The TXT defaults are 1048576 bytes, 65536 bytes per line, and empty acceptance.
`reject` checks zero bytes, not whitespace; the line limit has no CLI override.
TXT calls [`read_txt_with_options`](file-validation.md) directly, validating and
reading the same handle with metadata and actual-read size/empty checks. No
preflight/reopen is added. Pasted text and stdin retain their existing defaults.

For `parse` path input only, table options are trailing flag/value pairs in any
order: one `--header auto|none|row:N|search:N`, one `--include-rows LIST`, one
`--exclude-rows LIST`, and repeatable mixed `--sheet-name VALUE` /
`--sheet-index N`. A list is strict comma-separated `N` or inclusive `N-M`;
numbers are one-based ASCII decimal integers. Sheet names are exact Unicode
matches and selectors retain request order. Header/row options apply to CSV or
XLSX; sheet selectors apply only to XLSX. TXT, stdin/text and CSV sheet-selector
uses are usage errors. Duplicate singleton flags, malformed lists/numbers,
unknown/misplaced tokens and missing values are usage exit `2`, validated before
I/O. Well-formed semantic conflicts, missing targets and duplicate resolved
sheets are `table_selection_error` processing exit `1`.

No table option uses the legacy readers and emits byte-identical output. Any
table option selects the companion reader and adds optional table evidence.
`parse` precedence remains complete argv syntax/applicability, strict schema
decode, input extraction, shared compilation, then semantic selection.

Routing selects `.txt`, `.csv` or `.xlsx` case-insensitively on the final
extension. Unknown, absent and non-UTF-8 extensions fail before filesystem I/O,
without content sniffing. This deliberately changes missing unknown-extension
paths from `io_error` to `unsupported_input`, and directories with unsupported
extensions from `not_regular_file` to `unsupported_input`. Known TXT paths keep
the library's metadata/open/read order. For `parse`, complete syntax validation
precedes strict schema decoding, input extraction, shared compilation, then
optional semantic table selection;
an invalid schema can therefore still precede unsupported input routing.

Previously ignored tails in `parse`, schema inline text and schema compact
commands now fail usage/`2`. Help no longer attempts to open an inspect path.
The table-selection extension adds one typed processing-error code and optional
opt-in success evidence without changing JSON/package versions. Supported
no-option output and diagnostic redaction remain unchanged. CSV/XLSX table
CSV/XLSX path readers now apply their format-specific byte limits to bounded
same-handle reads; schema reads and parse/CLI result serialization are bounded as
described in the [resource-limit contract](data-contracts.md#resource-limits--implemented).
Post-materialization row/cell/result checks do not make file validation a sandbox
or stable snapshot; see its [filesystem limits](file-validation.md#compatibility-and-limits).

`ParseResponse` now includes canonical source evidence, unused content and
draft/review reasons. Its [additive compatibility contract](data-contracts.md#source-evidence-extension-and-compatibility)
distinguishes cell/string coordinates from original file bytes and legacy
responses without evidence. This does not establish complete integration
readiness. The reviewed [#12 shared schema compiler](data-contracts.md#executable-schema)
now supplies the same executable core plan to CLI and Rust callers. The local
[#13 text/name extension](data-contracts.md#contextual-textname-migration-13)
adds contextual fields and unresolved residuals. It is independently reviewed,
integrated and verified on macOS/Linux and in the batch container. Datetime
remains unsupported, and runtime/independence gates remain open.

## Proposed library caller experience

The intended integration is an independently reusable library with a small
generic call. No message queue or separately operated parser service is needed
or authorized for the initial integration. A bundled executable behind a JS
wrapper would still run locally with the caller, but the package must handle
native binaries, process lifecycle and temporary files. A WASM module could
avoid that plumbing; its actual byte-input and runtime gates remain open.

**Proposal only — not an existing npm API or executable example.** Exact adapter
types belong to #18 and must reuse the implemented shared schema contract and
[data contracts](data-contracts.md), not create a second schema model:

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

## Runtime evaluation — backend selected, adapter gates remain

[ADR 0005](decisions/0005-independent-engine-consumer-validation.md) retains CLI-first but
supersedes the unconditional process-next sequence in ADR 0004.
The bounded [#11](https://github.com/2001J/fuzzy-parser/issues/11) evaluation
retains a successful Node/CLI prototype and a WASM runtime experiment.
[ADR 0006](decisions/0006-library-interface-runtime-evaluation.md) owns the
comparison, selected library boundary and remaining adapter gates. [Dated CLI
evidence](evaluations/2026-08-28-node-cli.md) and [dated WASM evidence](evaluations/2026-08-30-wasm-runtime.md)
own reproducible
commands and measured results. No production adapter or Vercel deployment is
claimed. [#18](https://github.com/2001J/fuzzy-parser/issues/18) implements only
the reviewed choice after its engine prerequisites.

The prototype invokes the current CLI without copying schema conversion.
Its two fixture profiles demonstrate only supported integer/boolean subsets,
not the full #19 independence gate. WASM is a credible candidate, not rejected
because today's CLI was easier to exercise. [#22's byte-input API](data-contracts.md#xlsx-library-input--implemented)
and #12's shared schema compilation are implemented and independently verified
locally. #11's bounded JS/WASM comparison selects a single Node WASM package
with Worker isolation after independent review. A native
byte API and target compilation alone do not prove deployed JS/WASM behavior.
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

The 2026-08-28 read-only import preparation, with coordinator checks of the
critical paths at that same host commit, adds these migration constraints:

- The Contributor route treats only `mode=clean-sheet` as preparation; every
  other mode enters import. Do not send an invented `mode=preview` to it. A draft
  needs a distinct explicit contract, not a new label on the existing save path.
- Preparation has no imported-record or messaging effects, but
  [Event resolution](https://github.com/2001J/digital-invitation/blob/50fcaf072abd5307157ce1e0ee96676729e896c5/lib/event-context.ts)
  invokes [expired-Event archival](https://github.com/2001J/digital-invitation/blob/50fcaf072abd5307157ce1e0ee96676729e896c5/lib/event-lifecycle.ts).
  Thus a no-import-writes draft is not automatically a zero-database-writes HTTP
  guarantee. Confirmation must bind and revalidate Event, profile and reviewed rows.
- Contributor import can log a payment and automatically promote an eligible
  Contributor through the existing [promotion service](https://github.com/2001J/digital-invitation/blob/50fcaf072abd5307157ce1e0ee96676729e896c5/lib/contribution-promotion.ts),
  creating/updating a confirmed Guest without sending invitations. Preserve that
  host-owned behavior; the generic parser must never perform it.
- Guest preparation and upload differ in duplicate winners, phone checks and
  Contributor-conflict checks. A workbook labelled Upload Ready is not proof of
  save parity. Characterize those differences before a separate Guest cutover.
- Existing workbook selection/display formatting and TSV/delimited-TXT/XLS
  handling are not equivalent to the current engine. Keep those paths available
  until explicit host parity and rollback gates pass.

This preparation ran no host tests, builds, database/provider operations or
installation. Its proposed host tickets remain uncreated and implementation
requires a separately scoped host assignment.

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
| Opt-in Contributor profile and simple-CSV draft | Reviewed #11 runtime choice, #12/#13/#17/#18 and relevant #19 conformance | Real name/phone and required custom-field capability; contract/source-reference validation; no imported-record, promotion or messaging effects; legacy import unchanged |
| Contributor correction and review export | Draft bridge; #16 before broader table inputs | Original evidence beside corrections; unused/warning visibility; host validation; formula-safe exports and preserved custom fields; download/cancel without saving |
| Explicit Contributor confirmation | Reviewed draft and existing domain services | Server revalidation, auth/Event/profile/source binding, duplicates, qualification, replay/concurrency and partial outcomes; only explicitly confirmed valid rows persist |
| Contributor compatibility and default cutover | Prior slices and required #16/#19/#20 capabilities | All accepted extensions, header/sheet/highlight selection and typed/display values; explicit legacy exceptions, rollback and no silent fallback on engine failure |
| Separate Guest profile and cutover | Contributor evidence plus separate Guest decisions/tests | Status/category defaults, Event isolation, duplicate-winner and preparation/save differences, Contributor conflicts, global invitation identities and no sends |

The smallest meaningful first demo is a synthetic two-row CSV with names and
phones, source-backed proposed fields and review reasons, followed by cancel
with no import-model/provider calls. It waits for actual name/text support and
the reusable runtime; the current integer/boolean prototype is not a substitute.
Export follows review; saving belongs to the later explicit-confirmation slice.

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

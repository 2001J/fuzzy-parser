# Testing Strategy

Testing is part of each implementation ticket. The parser deals with ambiguous and malformed input, so examples that only cover happy paths are insufficient.
This is the required testing approach, not an inventory of tests already
implemented. See [current state](current-state.md) and the
[dated acceptance audit](audits/2026-08-27-backlog.md) for verified coverage.

The [CI guide](ci.md) owns automated gates, local commands and hosted-run limits.
CI guard tests live under `tools/ci/tests/` and run with Node's built-in test
runner, separately from the six Rust targets. Container checks assert actual
parse semantics, not just a successful `--help` exit.

## Test layers

### Unit tests

Use unit tests for isolated deterministic behavior:

- Schema validation.
- Error serialization.
- Whitespace and punctuation normalization.
- Delimiter scoring.
- Candidate detectors.
- Confidence calculations.
- Assignment scoring.

Unit tests should be small and explain one rule.

Keep unit-test bodies in each crate's `tests/unit/mod.rs`. The implementation
file includes that module with `#[cfg(test)]` and
`#[path = "../tests/unit/mod.rs"] mod tests;`, preserving private access through
`use super::*` without exposing implementation details as public API. Do not
create `tests/unit.rs` or `tests/unit/main.rs`: Cargo would discover an unwanted
standalone integration target. Existing CLI integration tests stay in
`tests/inspect.rs` and `tests/parse.rs`; synthetic fixtures stay in the repository
`fixtures/` directory. Anchor compile-time fixture includes with
`env!("CARGO_MANIFEST_DIR")` so module moves do not change their meaning.

For test-only relocations, compare `cargo test --workspace -- --list` per-target
names/counts and `cargo metadata --no-deps --format-version 1` targets before and
after the move. Preserve every test and assertion; account for feature-test
additions separately.

### Integration tests

Use integration tests for crate boundaries and complete library paths:

- Input adapter to canonical document.
- Canonical document through normalization.
- Schema-compiled text composition through reversible source mapping.
- Schema plus record candidate through assignment.
- Full parse request to parse result.

### CLI end-to-end tests

Execute the built CLI and verify:

- Arguments.
- Standard input.
- File input.
- Standard output.
- Standard error.
- Exit codes.
- JSON validity.

The CLI should be tested as a user experiences it, not only by calling internal functions.

### Isolated runtime evaluations

Keep exploratory transport harnesses outside production crates/packages.
The [#11 Node/CLI harness and dated evidence](evaluations/2026-08-28-node-cli.md)
exercise the existing executable with built-in Node assertions and synthetic
profiles under `fixtures/runtime/`. Run these checks separately from Cargo;
they add no Rust test target or implementation API. Two supported profiles in
this experiment alone does not complete the broader [#19 gate](#cross-profile-conformance-and-independence--implemented).
Record OS/architecture, runtime versions and the exact isolation tested;
local/container execution never substitutes for deployment evidence.

### Installable Node package

`node tools/ci/verify-node-package.mjs` is the deterministic #18 verification
entry point. It builds the pinned WebAssembly adapter, runs package tests and
TypeScript checks, packs and installs the tarball into a consumer without a
Rust toolchain on its `PATH`, then builds and invokes a generic Next.js
standalone fixture while checking Worker/WASM assets and hashes. Permanent tests
cover CJS/ESM parity, two unrelated profiles, all supported byte formats,
source-reference resolution, #17 exact boundaries, safe failures, missing or
corrupt assets, after-entry abort termination, end-to-end deadlines, recovery,
concurrency, determinism, and absence of sensitive logging. Combined with the
dedicated conformance corpus below, it completes #19; it is not publication or
Vercel deployment evidence.

### Fixture tests

Store synthetic source files under `fixtures/`.

The [TXT fixture inventory](../fixtures/text/README.md) records the permanent
Unicode/raw-whitespace, empty, blank-line, LF/CRLF, and invalid-UTF-8 cases for
#4. Its tests live in `parser-formats/tests/unit/txt_fixtures.rs`, nested under
the existing unit module without adding a Cargo target. Missing/directory path
tests and an injected permission-denied reader retain typed causes; they do not
depend on OS permissions or assert the error wire format owned by #2.

The [file-validation tests](../crates/parser-formats/tests/unit/file_validation.rs)
reuse TXT fixture setup and are included privately by the validation module,
without a new Cargo target. They check extension eligibility, metadata versus
actual reads, explicit empty policy, bounded growth, shrinkage and same-handle
extraction without timing races. Unix symlinks/sockets have platform-scoped tests.
Non-UTF-8 extension selection is tested with a synthetic OS path on Unix;
Linux additionally tests a real filename, which macOS does not permit creating.
The retained #2 CLI test separately checks real permission denial from a
non-root process. [New error tests](../crates/parser-core/tests/unit/file_validation_errors.rs)
assert exact safe/detailed output while the old cause and payload tests remain.
#5 deliberately changes the retained directory test to `not_regular_file` and
the CLI metadata-overflow expectation to `file_too_large`; bounded overflow
still returns `input_too_large`. See [compatibility](data-contracts.md#file-validation-additions-in-error-contract-01).

For XLSX byte input, `parser-formats` unit tests retain explicit file-reader
expectations and compare the byte-produced document, typed values, metadata,
errors and core source coverage. The existing CLI `inspect` test target checks
exact JSON parity without adding a serialization dependency to the formats crate.
[`unicode.xlsx.hex`](../fixtures/xlsx/unicode.xlsx.hex) is a hex-encoded synthetic
2,121-byte workbook, decoded in memory by the tests. It extends `sample.xlsx`
with a Unicode sheet name, Unicode/whitespace cell content and a formula `1+1`
whose cached value is deliberately `42`; the reader must preserve `42` rather
than evaluate the formula. No new ZIP-writing test dependency is needed.

The nested `resource_limits` modules exercise exact and one-over boundaries
without adding Cargo targets. Core tests cover every fixed resource wire name,
logical text/table record counts and compact serialized response bytes. Schema
tests cover bytes, fields, combined field/enum aliases, object and positional
encodings, string-safe nesting and structural/execution parity. Formats tests
cover CSV file/byte/document/table paths, blank logical rows, XLSX
file/byte/document/table paths, empty sheets, extracted cells and table-selection
record/response limits. CLI unit and subprocess tests cover bounded pretty JSON
including the trailing newline, schema file/stdin reads, processing exit `1`,
safe stderr and schema-before-input failure precedence.

These regressions deliberately test the contract boundaries without claiming
preallocation safety. CSV delimiter candidates exist before row/cell checks,
calamine worksheet ranges exist before XLSX cell checks, schema values exist
before field/alias checks, and parse responses exist before record/response-byte
checks. Tests assert the typed failure and observed count at those boundaries;
they do not claim ZIP expansion, dependency allocation or total process memory is
bounded by the corresponding configured value.

Current fixtures include the TXT inventory above, `csv/comma.csv`, `csv/messy.csv`,
`xlsx/sample.xlsx`, and schemas in `schema/`. The tree below illustrates desired
coverage and includes files not yet present; it is not acceptance evidence:

```text
fixtures/
├── text/
│   ├── simple.txt
│   ├── unicode.txt
│   ├── blank-lines.txt
│   └── multiline-record.txt
├── csv/
│   ├── comma.csv
│   ├── semicolon.csv
│   ├── quoted.csv
│   └── malformed.csv
├── xlsx/
└── schema/
```

Fixtures must not contain real private guest, customer, phone, or payment data.

### Golden or snapshot tests

Use stable expected JSON for public contracts and complete parse results.

Snapshots should exclude or normalize nondeterministic values such as processing duration or random identifiers.

Review snapshot changes as contract changes, not as automatic updates.

The #14 compatibility suite keeps the twelve supported-profile CLI/library
goldens byte-identical when `text_pipeline` is absent. Focused core, schema and
CLI tests cover CRLF/punctuation/trim/collapse mapping runs, UTF-8 boundaries,
blank source membership, synthetic-newline exclusion, repeated IDs and splits,
heading/competing boundary warnings, segment-local label scoring, singular
abstention, text/name exact values and constraints, strict nested option decoding,
safe capability failures, CLI/library parity, and unchanged CSV/XLSX content with
one ordered not-applied warning. The tests remain nested under the existing six
Cargo targets.

### Regression tests

#### Error contract regressions

[#2](https://github.com/2001J/fuzzy-parser/issues/2) separates compatibility into
legacy private-cause reads and exact versioned public-payload round trips. The
checked-in [legacy fixture](../fixtures/contracts/errors-legacy.json) preserves
all seven original format cause shapes/data; the
[pre-migration success goldens](../fixtures/contracts/cli-success-before-errors.json)
were captured and passed before serialization changed. CLI tests compare exact
success stdout with diagnostics both off and on. Existing #10 source/review
goldens and #22 path/byte cause tests remain unchanged.

Permanent core/schema tests cover default and detailed JSON/Display for all
format families, existing schema codes, all twelve validation reasons and the
additive output-serialization code. They check version acceptance/rejection,
private sentinel redaction, Unicode/control escaping, legacy deserialization,
new payload round trips, typed I/O conversion and actual nested schema causes.
Forged safe/detailed report messages and payload mutations verify that JSON,
`Display` and `message()` always use the current typed payload. Incoming outer
message text is ignored; explicit payload diagnostics retain exact round trips.
Format tests inject I/O failures without adding a JSON dependency to that crate.

Real CLI regressions cover absolute/missing/unreadable paths, malformed
CSV/XLSX/schema/UTF-8, numeric text limits, unsupported types, deterministic
messages, explicit versus incidental flag-like input, and exits/streams. Unix
permission tests run the subprocess without root privileges (drop to UID/GID
65534 if the test runner is root); non-UTF-8 OS argument tests retain the inspect
usage versus structured schema-input distinction. Test-only temporary files
are synthetic and removed by the shared `tests/support/mod.rs` helper; it is
not an additional Cargo target.

`output_serialization_error` targets and schema-serialization JSON causes are
tested at the typed report boundary: current concrete successful output models
do not provide a safe input that forces those serializer failures. Do not claim
those branches were induced through the CLI. Invalid schema serialization does
exercise the real `TargetSchema::to_json` validation cause.

The native Node evaluation retains exact direct/subprocess stderr parity and
adds version/redaction assertions. Container smoke uses exact safe error
envelopes; rebuild the binary/image before invoking these checks. Historical
runtime results are not evidence for changed code. See the authoritative
[migration contract](data-contracts.md#error-contract-01-and-migration-from-unversioned-errors).

#### Bug regressions

Every parser bug should produce a permanent regression test or fixture before or alongside the fix.

The test name should describe the failure, for example:

```text
does_not_split_phone_number_on_spaces
preserves_empty_middle_csv_cell
does_not_assign_unlabelled_integer_to_every_numeric_field
```

### Property-based tests

Property tests should verify broad invariants:

- Arbitrary Unicode does not panic.
- Repeated delimiters do not panic.
- Normalization is deterministic.
- Source spans remain within source bounds.
- Confidence remains between `0.0` and `1.0`.
- Parse results serialize and deserialize where supported.

### Fuzzing

Add fuzz targets after the relevant surfaces exist:

- TXT decoding and line extraction.
- CSV delimiter detection and parsing.
- Schema JSON decoding.
- Segmentation.
- Candidate detectors.

A fuzz-discovered crash must become a minimized regression fixture.

### Benchmarks

Benchmark only after behavior is correct and measurable.

Measure stages separately:

- Source extraction.
- Normalization.
- Segmentation.
- Candidate detection.
- Assignment.
- Serialization.

Suggested record sizes:

```text
100
1,000
10,000
100,000
```

Do not set performance promises before representative benchmarks exist.

## Required repository checks

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace
```

## Ticket-level test expectations

### Documentation-only

- Review the diff.
- Run `git diff --check` when a local checkout is available.
- Verify internal links and file names.
- Check the README, docs index, and authoritative topic documents for conflicting
  current/planned claims; preserve superseded decisions with an explanation.
- Run executable examples with synthetic input and assert values, record counts,
  warnings, source references, stdout/stderr, and exit codes. Exit `0` alone is
  not successful extraction. Clearly label templates/future commands that cannot
  be executed in the current environment.
- Before closing an old issue, compare every acceptance criterion with code and
  tests. Distinguish checked-in regression tests from temporary verification
  probes; keep missing durable coverage as explicit work.

### Model or serialization change

- Unit tests.
- JSON round-trip tests.
- Snapshot or golden contract tests.

### Input adapter change

- Unit tests for validation.
- Fixture-based extraction tests.
- Failure-case tests.
- Resource-limit tests.

### CLI change

- Binary integration tests.
- Exit-code assertions.
- stdout/stderr assertions.

The #16 table-selection regressions cover default byte parity, headerless and
preamble layouts, explicit/blank/typed/duplicate headers, bounded-search
match/no-match/tie behavior, inclusive include/exclude precedence and conflicts,
quote-aware CRLF/multiline blank CSV rows, Unicode selectors, empty and
unselected XLSX sheets, original/request order, typed/date/formula-cache values,
uninterpreted merged metadata, exact block references/coverage, every typed
selection reason, safe/detailed reports, and usage/processing exits.

The nested [argument regression module](../crates/parser-cli/tests/inspect/arguments.rs)
executes the real binary for all nine CLI forms, root/subcommand help, exact
arity, duplicate/malformed/misplaced flags, native OS arguments, leading-only
diagnostics, extension/error precedence and TXT size/empty overrides. Existing
inspect/parse success goldens and source checks remain in their original
targets. #6 changes former ignored-tail/bare-flag expectations to usage errors;
The [completed TXT fixture subprocess module](../crates/parser-cli/tests/inspect/txt_fixtures.rs)
adds the #7 Unicode/raw-whitespace and LF/CRLF/blank/trailing-terminator matrix,
plus invalid UTF-8 fixture materialization, without duplicating #6's retained
empty/size/extension/stream checks. Library growth, shrink and I/O tests remain
deterministic rather than timing subprocess races.

### Heuristic change

- Focused unit tests.
- Regression fixture.
- Comparison against nearby cases to prevent overfitting.

## Test data rules

- Use invented names and numbers.
- Do not commit customer exports.
- Do not commit production schemas containing sensitive labels or values.
- Keep malformed binary fixtures minimal and documented.
- Name fixtures by behavior, not by a real customer or event.

## Definition of done

A behavior ticket is complete when:

- The implementation compiles.
- Relevant unit and integration tests pass.
- Failure behavior is tested.
- Public behavior is documented.
- Existing fixtures remain green.
- No source data is silently discarded.

## Cross-profile conformance and independence — implemented

[#19](https://github.com/2001J/fuzzy-parser/issues/19) verifies engine independence with
synthetic text/TXT/CSV/XLSX fixtures and CLI/selected-boundary parity. The same
unmodified engine/public interface must process a synthetic QualEvents-shaped
profile and an unrelated supported-domain profile using caller configuration
only, with QualEvents not installed or available. Fixture profiles must remain
isolated from implementation; tests inspect dependency and runtime assumptions as
well as results. The native/CLI and installed CJS/ESM package suites now satisfy
this gate; see the [capability matrix](conformance.md).

Measure semantic output, source evidence and unresolved content; do not infer
accuracy from rule scores or claim unsupported field types work. Additional
generic capability coverage belongs in [#20](https://github.com/2001J/fuzzy-parser/issues/20).

Host review, export, auth/Event scope, duplicate policy, confirmed persistence,
and no-preview-side-effect tests belong to the future QualEvents task described
in [integration strategy](integration-strategy.md). Passing the Rust suite does
not establish those host guarantees. Host UI, adoption, migration and cutover
are not prerequisites for accepting an independently verified engine capability.

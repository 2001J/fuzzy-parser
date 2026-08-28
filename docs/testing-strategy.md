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
this experiment do not complete the broader [#19 gate](#cross-profile-conformance-and-independence--planned).
Record OS/architecture, runtime versions and the exact isolation tested;
local/container execution never substitutes for deployment evidence.

### Fixture tests

Store synthetic source files under `fixtures/`.

For XLSX byte input, `parser-formats` unit tests retain explicit file-reader
expectations and compare the byte-produced document, typed values, metadata,
errors and core source coverage. The existing CLI `inspect` test target checks
exact JSON parity without adding a serialization dependency to the formats crate.
[`unicode.xlsx.hex`](../fixtures/xlsx/unicode.xlsx.hex) is a hex-encoded synthetic
2,121-byte workbook, decoded in memory by the tests. It extends `sample.xlsx`
with a Unicode sheet name, Unicode/whitespace cell content and a formula `1+1`
whose cached value is deliberately `42`; the reader must preserve `42` rather
than evaluate the formula. No new ZIP-writing test dependency is needed.

Current fixtures include `text/simple.txt`, `csv/comma.csv`, `csv/messy.csv`,
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

### Regression tests

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

## Cross-profile conformance and independence — planned

[#19](https://github.com/2001J/fuzzy-parser/issues/19) gates engine readiness with
synthetic text/TXT/CSV/XLSX fixtures and CLI/selected-boundary parity. The same
unmodified engine/public interface must process a synthetic QualEvents-shaped
profile and an unrelated supported-domain profile using caller configuration
only, with QualEvents not installed or available. Fixture profiles must remain
isolated from implementation; inspect dependency and runtime assumptions as well
as results. This gate is planned, not satisfied by the existing suite or #10's
source-evidence regression tests.

Measure semantic output, source evidence and unresolved content; do not infer
accuracy from rule scores or claim unsupported field types work. Additional
generic capability coverage belongs in [#20](https://github.com/2001J/fuzzy-parser/issues/20).

Host review, export, auth/Event scope, duplicate policy, confirmed persistence,
and no-preview-side-effect tests belong to the future QualEvents task described
in [integration strategy](integration-strategy.md). Passing the Rust suite does
not establish those host guarantees. Host UI, adoption, migration and cutover
are not prerequisites for accepting an independently verified engine capability.

# Testing Strategy

Testing is part of each implementation ticket. The parser deals with ambiguous and malformed input, so examples that only cover happy paths are insufficient.

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

### Fixture tests

Store synthetic source files under `fixtures/`.

Suggested structure:

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
└── schemas/
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

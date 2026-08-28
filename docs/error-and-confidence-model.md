# Error And Confidence Model

The parser must separate unrecoverable failures from recoverable uncertainty.
This document owns diagnostic and confidence semantics. The requirements below
include planned behavior; use [data contracts](data-contracts.md) for actual
serialized shapes and [current state](current-state.md) for implementation gaps.

## Implemented boundary

- `parser-core::ParserError` has `io_error`, `invalid_utf8`, `unsupported_input`,
  `input_too_large`, `line_too_long`, `invalid_csv`, and `invalid_xlsx` variants.
  Missing files use `io_error` with `kind: "not_found"`.
- Format and schema causes now convert to `parser-core::Failure`, with shared
  typed payloads and exhaustive safe rendering. The [error contract and migration](data-contracts.md#error-contract-01-and-migration-from-unversioned-errors)
  owns the codes, exact fields and legacy-read versus new-wire compatibility.
  This #2 implementation is independently reviewed and verified locally.
- Normal processing failures use JSON stderr and exit `1`. Usage failures use
  plain stderr and exit `2`. Default JSON and `Display` omit supplied paths,
  caller values and opaque upstream prose. Explicit detailed reports may expose
  allowlisted caller context with JSON escaping; they are potentially sensitive,
  as are raw in-process cause fields and `Debug`. Do not send them to public logs.
- Assignment warnings include `required_field_missing` and
  `multiple_candidates_ambiguous`; separate segmentation APIs have boundary
  warnings. The document response now forwards input warnings before row-grouping
  warnings; record assignment warnings remain nested under each record.
  Excluded source blocks retain a reason. Aggregate confidence is not implemented.

## Record review

`RecordReview` is a deterministic summary of record-level evidence, not a
business validation or accuracy estimate. `needs_review` has one or more reasons
below, in this order; `draft` has none. Both require the host's review/confirmation
policy and neither authorizes persistence or messaging.

| Reason code | Trigger |
| --- | --- |
| `no_candidates` | No field candidates were detected, including empty records |
| `assignment_warnings` | The record's assignment emitted warnings |
| `unassigned_candidates` | Detected values remain unassigned |
| `unrecognized_content` | Non-whitespace content lies outside all detected spans |

Whitespace is retained in source coverage but does not alone trigger the last
reason. Scores do not determine these statuses. Document/input warnings still
need separate inspection even for a `draft` record. Missing review metadata in
legacy JSON means unavailable, not reviewed. See [data contracts](data-contracts.md)
for the serialized shape and source-accounting rules.

## Fatal errors

A fatal error prevents the requested parse operation from producing a trustworthy result.

Required categories (some remain unimplemented at the shared boundary):

- Unsupported input format.
- File not found or unreadable.
- File exceeds a configured hard limit.
- Invalid or unsupported encoding.
- Corrupt CSV or spreadsheet that cannot be safely extracted.
- Invalid schema.
- Internal invariant failure.

The implemented fatal-error contract contains stable codes, fixed human messages,
typed numeric/reason metadata and optional explicitly requested diagnostics.
It does not include retry policy or a generic invariant-failure category. Future
extensions may add:

- Additional typed source locations.
- Whether retrying with different input or configuration may help.

The CLI should return a non-zero exit code for fatal errors.

## Warnings

Warnings describe recoverable problems in the document, record, or field.

Examples of intended warning categories (not an implemented-code inventory):

- Ambiguous delimiter.
- Missing required field.
- Multiple candidates for one field.
- Low-confidence record boundary.
- Invalid candidate value.
- Unassigned fragment.
- Conflicting field values.
- Locale-dependent date ambiguity.
- Possible duplicate.

Warnings must not be represented only as prose. Use stable codes and structured metadata.

## Rejected fragments

A standalone rejected-fragment model remains proposed. Today's `ParseResponse`
accounts for every canonical block through embedded evidence, unused spans and
header/exclusion roles; unassigned detected candidates remain separate. See
[source coverage](data-contracts.md#versioned-parse-result). This is canonical
content retention, not exact original CSV/XLSX file fidelity or business rejection.

It should preserve:

- Raw content or a safe reference to it.
- Source location.
- Rejection reason.
- Confidence where relevant.

Rejected content must not disappear silently.

## Confidence layers

The target model distinguishes the levels below. Today field candidates and
separate segmentation results have scores/reasons; layered extraction,
assignment, and record aggregation are not implemented.

### Extraction confidence

How certain the adapter is that it decoded the source structure correctly.

Examples affecting it:

- Detected delimiter consistency.
- Workbook readability.
- Encoding certainty.

### Segmentation confidence

How certain the parser is that a proposed record begins and ends at the chosen boundaries.

Examples affecting it:

- Repeated row structure.
- Strong identifiers.
- Continuation-line evidence.
- Competing split or join interpretations.

### Candidate confidence

How certain the parser is that a span is a specific generic type.

Examples:

- A well-formed email has high email-candidate confidence.
- An unlabelled integer may have high numeric detection confidence but low semantic assignment confidence.

### Assignment confidence

How certain the parser is that a candidate belongs to a specific schema field.

Examples affecting it:

- Type compatibility.
- Nearby field label.
- Header mapping.
- Alias match.
- Uniqueness.
- Position.
- Caller-provided constraints.

### Record confidence

A summary of the record's overall reliability. It should consider the weaker stages rather than averaging away a critical uncertainty.

## Confidence scale

Current rules emit scores in the inclusive range `0.0` to `1.0`, but `Confidence`
is an `f64` alias and does not itself validate deserialized bounds. Scores are
heuristic evidence weights, **not calibrated accuracy probabilities**. For
example, `0.88` does not establish that 88% of those suggestions are correct.

Illustrative review bands for future evaluation, not shipped approval policy:

- `0.90–1.00`: strong evidence.
- `0.75–0.89`: likely correct but review may still be useful.
- `0.50–0.74`: ambiguous and should normally be reviewed.
- Below `0.50`: weak suggestion or unresolved result.

These bands have not been calibrated against an evaluation corpus. Consumers
must consider missing fields, ambiguity, source evidence, and business rules;
they may choose thresholds only with suitable validation. A high score never
authorizes persistence or messaging.

## Explainability

Every non-trivial score should include stable reason codes.

Example of current candidate evidence (excerpt, not a complete result):

```json
{
  "candidate_type": "phone_number",
  "confidence": 0.88,
  "reasons": [
    {
      "code": "phone_pattern_match",
      "message": "the value contains a plausible number of phone digits and valid separators"
    }
  ]
}
```

Additional negative-evidence categories are proposed, not current code names:

```text
MULTIPLE_COMPATIBLE_CANDIDATES
AMBIGUOUS_DATE_ORDER
WEAK_RECORD_BOUNDARY
CONFLICTING_LABEL
```

## Determinism

For the same parser version, input, schema, and options:

- Candidate sets should be stable.
- Assignments should be stable.
- Confidence should be stable.
- Reason codes should be stable.

Processing duration, generated identifiers, or unordered map traversal must not make snapshot tests nondeterministic.

## No fabricated certainty

The intended contract must prefer:

- `null` over invented data.
- Ambiguity over arbitrary tie-breaking.
- Warning over silent correction.
- Multiple candidates over a false single answer.

A consuming application may provide stricter rules, but the generic core must remain honest about evidence.
Current single-value assignment can select a candidate even when alternatives
exist, while returning an ambiguity warning. Consumers must not treat that
selection as resolved certainty or assume the engine abstains in every tie.

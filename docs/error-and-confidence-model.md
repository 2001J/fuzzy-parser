# Error And Confidence Model

The parser must separate unrecoverable failures from recoverable uncertainty.

## Fatal errors

A fatal error prevents the requested parse operation from producing a trustworthy result.

Examples:

- Unsupported input format.
- File not found or unreadable.
- File exceeds a configured hard limit.
- Invalid or unsupported encoding.
- Corrupt CSV or spreadsheet that cannot be safely extracted.
- Invalid schema.
- Internal invariant failure.

Fatal errors should include:

- Stable machine-readable code.
- Human-readable message.
- Optional source location.
- Optional safe diagnostic context.
- Whether retrying with different input or configuration may help.

The CLI should return a non-zero exit code for fatal errors.

## Warnings

Warnings describe recoverable problems in the document, record, or field.

Examples:

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

A rejected fragment is source content that could not be incorporated into a record or assignment.

It should preserve:

- Raw content or a safe reference to it.
- Source location.
- Rejection reason.
- Confidence where relevant.

Rejected content must not disappear silently.

## Confidence layers

Confidence is represented at several levels:

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

All confidence scores use the inclusive range `0.0` to `1.0`.

Suggested interpretation:

- `0.90–1.00`: strong evidence.
- `0.75–0.89`: likely correct but review may still be useful.
- `0.50–0.74`: ambiguous and should normally be reviewed.
- Below `0.50`: weak suggestion or unresolved result.

These bands are product guidance, not universal truth. Consuming applications may choose their own review thresholds.

## Explainability

Every non-trivial score should include stable reason codes.

Example:

```json
{
  "field": "phone",
  "confidence": 0.98,
  "reasons": [
    { "code": "PHONE_PATTERN_MATCH" },
    { "code": "ONLY_COMPATIBLE_CANDIDATE" },
    { "code": "CALLER_COUNTRY_HINT_APPLIED" }
  ]
}
```

Negative evidence should also be recorded:

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

The parser must prefer:

- `null` over invented data.
- Ambiguity over arbitrary tie-breaking.
- Warning over silent correction.
- Multiple candidates over a false single answer.

A consuming application may provide stricter rules, but the generic core must remain honest about evidence.

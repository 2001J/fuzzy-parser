# Parsing Pipeline

This document defines the responsibilities and invariants of each parser stage.

## 1. Input acquisition

Sources may arrive as pasted text, standard input, or uploaded files.

Responsibilities:

- Identify the declared source type.
- Capture file name, size, MIME hint, and other available metadata.
- Enforce source-level limits before expensive processing.

Must not:

- Infer fields.
- Normalize content.
- Execute formulas, macros, scripts, or links.

## 2. Format extraction

A source-specific adapter converts the input into a canonical raw document.

Examples:

- TXT: preserve lines and line numbers.
- CSV: preserve rows, columns, delimiter choice, quoted values, and header candidates.
- XLSX: preserve workbook, sheet, row, column, cell type, and stored/displayed values where safely available.

Output requirements:

- Raw content remains unchanged.
- Every block has a stable identifier.
- Every block records its source location.
- Extraction warnings are represented rather than hidden.

Extraction does not decide what a record or field means.

## 3. Normalization

Normalization produces a derived representation while preserving the raw source.

Possible transforms:

- Line-ending normalization.
- Leading and trailing whitespace trimming.
- Repeated whitespace collapsing.
- Visually equivalent dash or quote normalization.
- Unicode normalization where explicitly chosen.
- Detection of list markers, timestamps, headings, or sender prefixes.

Rules:

- Every transform must be deterministic.
- Destructive transforms must be recorded.
- Potential noise should initially be marked, not silently deleted.
- Normalization must be configurable where meaning may change.

## 4. Record segmentation

Segmentation proposes which blocks or spans belong to one logical record.

Strategies may include:

- One meaningful line per record.
- One table row per record.
- Multiline continuation.
- Multiple records in one line when repeated strong identifiers exist.
- Header and section detection.

Output:

- One or more `RecordCandidate` values.
- Source block references.
- Segmentation confidence.
- Reasons for joins or splits.
- Warnings for ambiguous boundaries.

The implemented repeated-identifier strategy uses only strong, configured label markers. It splits a block only when one marker repeats from the beginning with non-empty values; near misses and competing marker sets remain intact, with a warning when the boundary is ambiguous. Heading-marked blocks are preserved as visible boundaries, and indented text after a heading is kept separate with a low-confidence warning because section content is not automatically a record. Segmentation must not fabricate field values.

## 5. Field candidate detection

Detectors identify generic value candidates without assigning business meaning.

Initial detector types may include:

- Phone number.
- Email.
- Integer and decimal.
- Currency.
- Date and time.
- Boolean.
- Caller-defined enum alias.
- Residual text.
- Person-name candidate.

Email, integer, decimal, phone-number, boolean, date, currency, and caller-defined enum detection are currently implemented with conservative whole-token matching. These detectors preserve raw values, provide normalized values where safe, and report byte-accurate source spans.

Each candidate records:

- Raw value.
- Optional normalized value.
- Candidate type.
- Source span.
- Confidence.
- Evidence or reason codes.

Multiple candidates of the same type are valid.

The current assignment slice matches candidate types against caller-provided field definitions and uses nearby canonical or caller-provided labels as context. Single-value fields prefer a context-matched candidate, then select the highest-confidence match and report ambiguity when multiple matches remain; multiple-value fields retain all compatible matches. Required fields without a compatible candidate and candidates left unassigned are reported without fabricating values.

## 6. Schema-driven assignment

The caller-provided schema describes the desired fields. Assignment scores compatible candidates against those fields.

Possible evidence:

- Type compatibility.
- Explicit nearby label.
- Caller-provided alias.
- Relative position.
- Uniqueness within the record.
- Header-to-column mapping.
- Locale or country hint.
- Caller-provided constraints.

Assignment rules:

- Missing values remain missing.
- Multiple plausible assignments remain ambiguous unless evidence separates them.
- Unassigned candidates are returned.
- A low-confidence assignment must be distinguishable from a high-confidence assignment.
- The engine must not invent values to satisfy required fields.

## 7. Validation

Validation checks assigned records against the schema and generic rules.

Examples:

- Required field missing.
- Value outside an allowed range.
- Invalid enum value.
- Multiple values assigned to a singular field.
- Cross-field caller constraint failure.
- Normalization failure.

Validation produces structured warnings or record errors. It does not trigger product side effects.

## 8. Confidence and explanation

Confidence is layered rather than represented as one unexplained number:

- Extraction confidence.
- Segmentation confidence.
- Detection confidence.
- Assignment confidence.
- Record confidence.

Every score should be reproducible from documented factors. Explanations should use stable reason codes plus readable text.

## 9. Result construction

The final parse result includes:

- Parsed record candidates.
- Assigned and unassigned field candidates.
- Raw source references.
- Confidence values.
- Warnings and errors.
- Rejected fragments.
- Processing statistics.
- Parser and schema contract versions.

No fragment should disappear merely because the parser did not understand it.

## 10. Review and confirmation

Review belongs to the caller or standalone interface, not the parser core.

A review surface may:

- Edit suggestions.
- Approve or reject rows.
- Merge or split records.
- Resolve duplicate candidates.
- Inspect source evidence.
- Export confirmed results.

The parser itself returns candidates. It does not declare that uncertain human-created data is automatically correct.

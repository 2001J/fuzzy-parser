# Parsing Pipeline

This document defines the responsibilities and invariants of each parser stage.
It describes the intended complete pipeline. The current paths are narrower:

| Entry point | Stages used today |
| --- | --- |
| CLI `inspect` | Format extraction → canonical `RawDocument` JSON |
| `normalize_document` / `segment_document` | Separately callable normalization and segmentation APIs |
| `parse_text_with_assignment` | Detectors → assignment for one supplied text record |
| CLI `parse` / `parse_document_with_assignment` | Table grouping/header detection/row assignment, or independent raw-block text assignment |

`parse` does not currently compose normalization or record segmentation.
[#14](https://github.com/2001J/fuzzy-parser/issues/14) owns that connection;
the #10 result extension retains canonical source evidence and unused content.
Public shapes and coordinate conventions belong in
[data contracts](data-contracts.md), not this stage description.

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
- CSV: preserve extracted cell values, row/column coordinates, and delimiter choice;
  header interpretation belongs to the core, not the adapter.
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

Email, integer, decimal, phone-number, boolean, date, currency, and caller-defined enum detection are currently implemented with conservative whole-token matching. These detectors preserve raw values, provide normalized values where safe, and report byte spans in the text passed to the detector. For table parsing that text is a derived row string, not original file bytes.

Each candidate records:

- Raw value.
- Optional normalized value.
- Candidate type.
- Source span.
- Confidence.
- Evidence or reason codes.

Multiple candidates of the same type are valid.

The current assignment slice matches candidate types against caller-provided field definitions and uses nearby canonical or caller-provided labels, source-column metadata, or detected table-header labels as context. Integer and length constraints filter incompatible candidates before selection. Single-value fields prefer a context-matched candidate, then select the highest-confidence match and report ambiguity when multiple matches remain; multiple-value fields retain all compatible matches, narrowed to header-matching columns when a header context exists and at least one column matches. Required fields without a compatible candidate and candidates left unassigned are reported without fabricating values.

For tabular documents, `group_document_rows` groups blocks carrying row provenance into per-sheet rows. Blocks without row metadata are excluded from grouping with a warning; their raw values remain only in the input document. `detect_table_headers` requires at least two rows and at least two non-empty text cells in the first row without strongly typed values. Rejections carry `header_not_detected_*` codes, but an all-text data row can still be mistaken for a header. `parse_document_rows_with_assignment` composes grouping, header detection, per-row detection with one-based source columns, and assignment that records `header_label_match`. Explicit header/selection control is planned in [#16](https://github.com/2001J/fuzzy-parser/issues/16).

`parse_text_with_assignment` provides the deterministic composition point for a text record: it runs the built-in detectors, applies caller-defined enum definitions, and returns both the complete candidate evidence and the assignment result. Callers can still invoke each stage independently when they need custom ordering or format-specific provenance.

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

Validation is intended to produce structured warnings or record errors. Today
assignment checks required fields and integer/length constraints and reports
ambiguity. Generic draft/review reasons now summarize these warnings and unused
content. Arbitrary cross-field constraints are not implemented; business
approval/rejection remains host-owned. Parsing does not trigger product side effects.

## 8. Confidence and explanation

The intended confidence model distinguishes these layers:

- Extraction confidence.
- Segmentation confidence.
- Detection confidence.
- Assignment confidence.
- Record confidence.

Today candidates and separate segmentation results carry heuristic scores and
reason codes; aggregate record confidence is not implemented. These scores are
not calibrated accuracy probabilities. The authoritative semantics and current
limitations are in [error and confidence model](error-and-confidence-model.md).

## 9. Result construction

The intended complete result includes:

- Parsed record candidates.
- Assigned and unassigned field candidates.
- Raw source references.
- Confidence values.
- Warnings and errors.
- Rejected fragments.
- Processing statistics.
- Parser and schema contract versions.

No fragment should disappear merely because the parser did not understand it.
Current `ParseResponse` retains the canonical source document, noncandidate
content, assigned/unassigned references, header/exclusion evidence, record review
reasons, and input-document warnings. Statistics and a shared request/schema
compilation interface remain unimplemented. This does not change extraction,
header, detection or segmentation heuristics or preserve original file bytes
that the extraction adapters do not expose.
See [the implemented result contract](data-contracts.md#versioned-parse-result).

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

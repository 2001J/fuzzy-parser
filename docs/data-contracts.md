# Data Contracts

This document owns public model shapes. Sections marked **proposed** are design
sketches, not available APIs. Other models reflect the Rust sources in
[`parser-core`](../crates/parser-core/src/lib.rs) and
[`parser-schema`](../crates/parser-schema/src/lib.rs). Serialized meanings must
change deliberately and with compatibility tests; Rust-looking sketches alone
are not proof that a feature exists.

Public contracts belong to the independent engine: raw input, caller-owned
schema/options, generic values, evidence and uncertainty. They must not introduce
QualEvents domain types or require that application to compile or run. Synthetic
consumer profiles are isolated test inputs; the planned cross-profile gate is
defined in [testing strategy](testing-strategy.md).

## Contract versioning

The implemented `ParseResponse.contract_version` is `0.1`; schema JSON uses
`schema_version: "0.1"`; the workspace/package and response `parser_version` are
`0.1.0`. Raw inspection and current error envelopes have no version field.
A unified serialized parse request is still proposed. Planning milestone names
are not any of these versions; see [release strategy](release-and-environment-strategy.md).

```json
{
  "contract_version": "0.1"
}
```

Parser implementation version and schema contract version are separate concerns.

### Source-evidence extension and compatibility

#10 extends JSON contract `0.1` additively; it does not rename existing fields,
change candidate `source_span` meanings, or bump package/schema versions.
New document parses always include `source_evidence`, candidate
`source_reference` values and record `review` metadata. These optional Rust
fields default to `None` when reading older JSON and are omitted when absent.
Missing evidence means **unavailable**, not an empty or reviewed source.

Tolerant existing JSON readers can ignore the additions. Readers that reject
unknown keys must update their accepted shape before consuming the extended
output. The [legacy golden](../fixtures/contracts/parse-0.1.json) round-trips
without additions; CLI tests compare every legacy field after removing only the
new keys. The [review golden](../fixtures/contracts/parse-source-review.json)
fixes the extended envelope's shape. `inspect` and canonical raw JSON are unchanged.

Rust function signatures remain unchanged, but manual struct literals need the
new optional fields: `FieldCandidate.source_reference`, `TextParseResult.review`,
`ParseResponse.source_evidence`, and `TableCell.source_block_index`. Use `None`
when no evidence exists; do not invent references. Parsing entry points populate
these fields. This is JSON compatibility, not a claim that old Rust struct
literals compile unchanged.

## Parser input — proposed

The following `ParserInput` does not exist. Current `parser-formats::InputSource`
supports borrowed text, a stdin reader, or a TXT path; CSV and XLSX have separate
reader functions. The CLI dispatches among those readers.

```rust
pub enum ParserInput {
    Text { content: String },
    File { path: PathBuf },
    Table { rows: Vec<Vec<RawValue>> },
}
```

A public API may represent files differently for browser or service use, but every surface must converge on the same canonical document model.

## XLSX library input — implemented

[`parser-formats`](../crates/parser-formats/src/lib.rs) exposes two entry points:

```rust
pub fn read_xlsx(path: impl AsRef<Path>) -> Result<RawDocument, ParserError>;
pub fn read_xlsx_bytes(file_name: Option<&str>, bytes: &[u8]) -> Result<RawDocument, ParserError>;
```

The byte API borrows an XLSX archive and reads it in memory. `file_name` is opaque
caller metadata, never a path to open; it is preserved verbatim, including
Unicode, or remains `None`/JSON `null`. It does not create files, access networks
or evaluate formulas, macros or external links. The existing path API still
uses calamine's buffered file reader, without copying the entire archive into
a byte vector. Both use one cell-extraction/mapping path.

With equivalent filename metadata, both return identical canonical documents
and JSON: `xlsx` source type, workbook byte size, XLSX MIME type, no delimiter,
ordered block IDs, stored typed values, blank cells, worksheet coordinates and
warnings. Cached formula values are read without recalculation. This does not
add original-file byte offsets, displayed formatting or new table-selection rules.

Missing files and invalid file reads preserve the existing error categories,
supplied paths and messages. Invalid byte input returns `InvalidXlsx`
(`invalid_xlsx`) with `path: ""` and the generic message
`could not read XLSX workbook`; no filename, sheet name or workbook content is
added to diagnostics. The empty string preserves the existing required string
field without inventing a filesystem path. Shared file-error redaction remains
[#2](https://github.com/2001J/fuzzy-parser/issues/2).

This additive library API does not change CLI commands or JSON/package versions.
The local [#22](https://github.com/2001J/fuzzy-parser/issues/22) implementation and
file-reader parity are independently verified. Workbook/decompression/cell/output limits remain
[#17](https://github.com/2001J/fuzzy-parser/issues/17); WASM/JS execution, shared
schema compilation and runtime packaging are separate, unverified capabilities.

## Canonical raw document

```rust
pub struct RawDocument {
    pub id: String,
    pub source: SourceMetadata,
    pub blocks: Vec<RawBlock>,
    pub warnings: Vec<ParserWarning>,
}

pub struct SourceMetadata {
    pub source_type: SourceType,
    pub file_name: Option<String>,
    pub mime_type: Option<String>,
    pub size_bytes: Option<u64>,
    pub delimiter: Option<String>,
}

pub struct RawBlock {
    pub id: String,
    pub value: RawValue,
    pub location: SourceLocation,
}

pub struct SourceLocation {
    pub line: Option<usize>,
    pub row: Option<usize>,
    pub column: Option<usize>,
    pub sheet: Option<String>,
    pub byte_start: Option<usize>,
    pub byte_end: Option<usize>,
}
```

`RawValue` must preserve source meaning. A numeric spreadsheet cell should not be silently converted into an irreversible string if the source type is known. Implemented raw value variants include text, integer, decimal, boolean, date-time serial/text, duration, error, and null.

JSON `RawValue` is tagged with `kind` and `value`, for example
`{"kind":"Text","value":"Ada"}`; variant names retain Rust casing.
`SourceType` uses `text`, `stdin`, `txt`, `csv`, and `xlsx`. Optional metadata
serializes as `null`. A document ID/block ID is local to that document, not a
globally unique import identifier.

TXT lines and table row/column coordinates are one-based. TXT block byte ranges
are zero-based, end-exclusive and exclude line terminators; blank lines are
blocks, a final terminator adds no extra block, and an empty file has zero blocks.
CSV coordinates describe parsed records/cells, not original file byte offsets;
CSV blank physical lines are not represented. XLSX uses worksheet coordinates
within extracted ranges, with stored typed values, not a complete display/style
model. Canonical extracted values do not preserve every byte of the original
CSV/XLSX file. Callers needing exact original files must retain them safely.

## Normalized block

```rust
pub struct NormalizedBlock {
    pub source_block_id: String,
    pub original: RawValue,
    pub normalized_text: String,
    pub transformations: Vec<Transformation>,
}

pub enum Transformation {
    LineEndingsNormalized,
    WhitespaceTrimmed,
    WhitespaceCollapsed,
    DashesNormalized,
    QuotesNormalized,
    ListMarkerDetected,
    TimestampPrefixDetected,
    SenderPrefixDetected,
    HeadingDetected,
}
```

Normalization derives a representation. It never replaces the raw source. Noise detections are recorded as transformations and do not delete source prefixes.

## Record candidate

```rust
pub struct RecordCandidate {
    pub id: String,
    pub source_block_ids: Vec<String>,
    pub text: String,
    pub confidence: Confidence,
    pub reasons: Vec<Reason>,
    pub warnings: Vec<ParserWarning>,
}
```

A record may originate from one source block or several joined blocks.

The segmentation API emits heuristic `Confidence` values and reason metadata.
`Confidence` is a type alias, not a type enforcing bounds during deserialization:

```rust
type Confidence = f64;

pub struct Reason {
    pub code: String,
    pub message: String,
}

pub enum SegmentationStrategy {
    OneBlockPerRecord,
    OneRowPerRecord,
    JoinIndentedContinuations,
    SplitRepeatedIdentifiers,
}
```

Current segmentation proposes one-block, one-row, indented-continuation, conservative repeated-identifier, and heading-aware boundaries. `SplitRepeatedIdentifiers` recognizes repeated generic markers such as `ID:`, `Record:`, and `Item:`; callers can provide other marker labels through `segment_document_with_repeated_identifier_markers`. Ambiguous cases such as mixed marker sets and preambles remain intact with an ambiguity warning; ordinary near misses do not split. Heading-marked blocks remain observable candidates, and indented text immediately following a heading remains separate with an ambiguity warning.

## Target schema

```rust
pub struct TargetSchema {
    pub schema_version: String,
    pub record_name: Option<String>,
    pub fields: Vec<FieldDefinition>,
    pub options: SchemaOptions,
}

pub struct FieldDefinition {
    pub name: String,
    pub field_type: FieldType,
    pub required: bool,
    pub multiple: bool,
    pub aliases: Vec<String>,
    pub constraints: Vec<FieldConstraint>,
}
```

Initial generic field types:

```text
text
person_name
phone_number
email
integer
decimal
currency
date
datetime
boolean
enum
```

Product-specific concepts should be represented as generic fields plus caller-provided enum values, aliases, and constraints.

`TargetSchema::from_json` validates a JSON schema before returning it, including the supported schema version and unambiguous field and enum labels. `TargetSchema::to_json` refuses to serialize an invalid schema. Parsing and validation errors remain structured as `SchemaParseError` values.

Validation accepts the field vocabulary above; CLI execution rejects `text`,
`person_name`, and `datetime`. Enum JSON uses an externally tagged object such as
`{"enum":{"values":[{"value":"active","aliases":["enabled"]}]}}`.
Constraints are tagged with `kind`/`value`; see the source for exact variants.
`SchemaOptions` currently contains only `allow_unknown_fields`, which the CLI
parse path does not enforce. Locale/country hints, runtime options, and explicit
column mapping are not supported schema fields. The CLI pools enum values
across fields; shared capability/field-scope enforcement is [#12](https://github.com/2001J/fuzzy-parser/issues/12).

The CLI preserves this validation contract for file, standard-input, and inline-text sources. Valid output is pretty-printed by default and can be emitted as one compact JSON line for pipeline consumers.

## Field candidate

```rust
pub struct FieldCandidate {
    pub candidate_type: CandidateType,
    pub raw_value: String,
    pub normalized_value: Option<serde_json::Value>,
    pub source_span: TextSpan,
    pub source_column: Option<usize>,
    pub source_reference: Option<SourceReference>,
    pub confidence: Confidence,
    pub reasons: Vec<Reason>,
}
```

Candidates are evidence. They are not automatically assignments.
`source_span` is a zero-based, end-exclusive byte range in the detector's input
text. In table mode, that input is the concatenation of trimmed non-empty cells
with spaces. This legacy span remains unchanged for assignment compatibility;
it is not an original-cell or file-byte offset. `source_column` identifies the
cell column. The new `source_reference` resolves to a stored canonical value:

```rust
pub struct SourceReference {
    pub block_index: usize,
    pub coordinate_space: SourceCoordinateSpace,
    pub span: TextSpan,
}
```

`block_index` is zero-based in `source_evidence.document.blocks`, disambiguating
even repeated caller block IDs. Both span ends count UTF-8 bytes, with an
exclusive end. `raw_text_utf8` indexes the unchanged stored string of `Text`,
`DateTimeText`, `Duration` or `Error`; `rendered_value_utf8` indexes
`RawValue::to_text()` for integer, decimal, boolean, date serial or null values.
The original typed value remains embedded. Neither coordinate space claims
CSV/XLSX file-byte offsets. For TXT, a caller can combine a raw-text span with
the known block file offset; table locations instead identify sheet/row/column.

For example, the email in `fixtures/csv/source-review.csv` has a legacy row span
`5..21` but references block index `5`, raw-text bytes `2..18` of the cell
`"  ada@example.test  "`. Trimming never overwrites the cell or its whitespace.
`SourceReference::resolve(&document)` returns that substring, or `None` for an
invalid index, coordinate kind, range or UTF-8 boundary.

References are attached before assignment, so detector, assigned and unassigned
copies all retain them (assignment may add reasons). Standalone detectors and
`parse_text_with_assignment` have no canonical document and omit references.
The lower-level table API references its caller's original document; use the
document-level entry point for a self-contained response.

## Field assignment

```rust
pub struct AssignmentField {
    pub name: String,
    pub aliases: Vec<String>,
    pub candidate_type: CandidateType,
    pub required: bool,
    pub multiple: bool,
    pub unique: bool,
    pub constraints: Vec<AssignmentConstraint>,
    pub expected_column: Option<usize>,
}

pub struct AssignedField {
    pub name: String,
    pub candidates: Vec<FieldCandidate>,
}

pub struct AssignmentResult {
    pub fields: Vec<AssignedField>,
    pub unassigned_candidates: Vec<FieldCandidate>,
    pub warnings: Vec<ParserWarning>,
}
```

`assign_candidates` matches candidate types against caller-provided fields, applies integer and length constraints, uses nearby labels and optional expected-column metadata as context, and serializes its result for integration surfaces. A missing value is different from an empty string. Ambiguous assignments and unassigned candidates remain observable.

`TextParseResult` contains candidates, `AssignmentResult`, and optional
`RecordReview` (`status` and `reasons`). The current parser populates review
metadata; legacy JSON may omit it. [Diagnostic semantics](error-and-confidence-model.md#record-review)
define the draft/review states; neither is approval or an accuracy probability.

## Tabular header context and row assignment

```rust
pub struct TableCell {
    pub source_column: usize,
    pub value: RawValue,
    pub source_block_id: String,
    pub source_block_index: Option<usize>,
}

pub struct TableRowGroup {
    pub sheet: Option<String>,
    pub source_row: usize,
    pub cells: Vec<TableCell>,
    pub source_block_ids: Vec<String>,
}

pub struct TableHeaderContext {
    pub sheet: Option<String>,
    pub source_row: usize,
    pub labels: Vec<(usize, String)>,
    pub source_block_ids: Vec<String>,
}

pub struct TableParseResult {
    pub sheets: Vec<SheetTableResult>,
    pub warnings: Vec<ParserWarning>,
}

pub struct SheetTableResult {
    pub sheet: Option<String>,
    pub header: HeaderExtraction,
    pub records: Vec<TableRowParseResult>,
}

pub enum HeaderExtraction {
    Detected { headers: Box<TableHeaderContext> },
    NotDetected { code: String, message: String },
}
```

`TableRowParseResult` carries `source_row`, `source_block_ids`, and `parse`.
`HeaderExtraction` serializes with a snake-case `status` tag: `detected` or
`not_detected`. The [pipeline](parsing-pipeline.md) owns header-detection and
assignment behavior. The lower-level table result does not embed its input;
the document-level response below retains header, parsed and excluded blocks.

## Versioned parse result

The document-level entry point returns one versioned envelope that covers both tabular and unstructured inputs.

```rust
pub struct ParseResponse {
    pub contract_version: String,
    pub parser_version: String,
    pub record_name: Option<String>,
    pub source_type: SourceType,
    pub content: ParseContent,
    pub warnings: Vec<ParserWarning>,
    pub source_evidence: Option<SourceEvidence>,
}

#[serde(tag = "mode", rename_all = "snake_case")]
pub enum ParseContent {
    Table { sheets: Vec<SheetTableResult> },
    Text { records: Vec<TextRecordParseResult> },
}

pub struct TextRecordParseResult {
    pub source_block_id: String,
    pub parse: TextParseResult,
}
```

`parse_document_with_assignment` chooses table mode when any blocks have row
provenance, otherwise one text record per raw block. It does not run the separate
normalization/segmentation APIs. The CLI emits this envelope directly.

The envelope now embeds the unchanged canonical document, including metadata,
typed values, blank cells/lines, header values, noncandidate text and input
warnings. Top-level warnings contain input-document warnings first, then row
grouping warnings. Record assignment warnings remain under
`parse.assignment.warnings`; they are not silently promoted to fatal errors.

```rust
pub struct SourceEvidence {
    pub document: RawDocument,
    pub blocks: Vec<SourceBlockCoverage>,
}

pub struct SourceBlockCoverage {
    pub block_index: usize,
    pub role: SourceBlockRole,
    pub coordinate_space: SourceCoordinateSpace,
    pub unused_spans: Vec<TextSpan>,
    pub reason: Option<Reason>,
}
```

There is exactly one coverage entry per block in original order:

- `parsed`: `unused_spans` complement the union of all detected candidate spans
  in the stored/rendered value. They retain whitespace and unrecognized text;
  an empty value has `0..0`. Overlapping detections are allowed. Detected but
  unassigned values stay in `assignment.unassigned_candidates` rather than
  being mislabeled as undetected content.
- `header`: the whole block is retained as header evidence with
  `header_detected`. The existing heuristic can still misclassify a data row;
  this role exposes its decision, not proof that the row is a header.
- `excluded`: the whole block remains available with `row_provenance_missing`
  when mixed table/text input excludes a block without row metadata.

Header/excluded entries have no unused spans because their role accounts for
the entire value. There is no new business rejection policy or hidden deletion.
Every canonical value is retained; adapters still omit details such as CSV
blank physical lines, original quoting and XLSX styles outside their existing
extraction contract. Callers needing original files must retain them separately.
Output now includes potentially sensitive raw input: do not log it by default
or expose it to readers without permission to inspect the source. Redacted
output modes and broader resource limits remain future work.

## Parsed record — proposed

This alternative record shape is not the implemented `RecordReview` extension.
Aggregate record confidence and the statuses below remain proposed.

```rust
pub struct ParsedRecord {
    pub id: String,
    pub source_record_id: String,
    pub fields: Vec<FieldAssignment>,
    pub unassigned_candidates: Vec<FieldCandidate>,
    pub confidence: Confidence,
    pub status: RecordStatus,
    pub warnings: Vec<ParserWarning>,
}
```

Suggested statuses:

```text
clean
needs_review
invalid
rejected
```

The consuming application decides whether a status blocks import.

## Parse request — proposed

```rust
pub struct ParseRequest {
    pub input: ParserInput,
    pub schema: TargetSchema,
    pub options: ParseOptions,
}
```

Options may include:

- Locale.
- Default country code.
- Segmentation strategy.
- Normalization toggles.
- Fuzzy-match thresholds.
- Resource limits.
- Diagnostic verbosity.

Defaults must be safe and documented.

## Parse result — proposed

```rust
pub struct ParseResult {
    pub contract_version: String,
    pub parser_version: String,
    pub records: Vec<ParsedRecord>,
    pub rejected_fragments: Vec<RejectedFragment>,
    pub warnings: Vec<ParserWarning>,
    pub statistics: ParseStatistics,
}
```

Suggested statistics:

- Source blocks read.
- Record candidates produced.
- Clean records.
- Records needing review.
- Invalid records.
- Rejected fragments.
- Unassigned candidates.
- Processing duration.

Duration is diagnostic metadata and must not affect deterministic output comparisons.

## Serialization requirements

These are evolution requirements, not a claim that every current input or
output validates them. In particular, safe diagnostic redaction, arbitrary
normalization source maps and adapter-level original-file fidelity remain open work.

- Use explicit enums rather than magic strings internally.
- JSON field names must remain stable once external integrations depend on them.
- Unknown future fields should not break tolerant readers where possible.
- Errors and warnings use machine-readable codes.
- Source locations use a clearly documented indexing convention.
- Floating confidence values must have defined bounds and meaning.
- Sensitive raw input should be optionally omitted from low-privilege output modes while preserving source identifiers.

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
    pub confidence: Confidence,
    pub reasons: Vec<Reason>,
}
```

Candidates are evidence. They are not automatically assignments.
`source_span` is a zero-based, end-exclusive byte range in the detector's input
text. In table mode, that input is the concatenation of trimmed non-empty cells
with spaces; it is not returned as a source map. `source_column` identifies the
cell column, but exact original-cell offsets still need
[#10](https://github.com/2001J/fuzzy-parser/issues/10).

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

`TextParseResult` contains the candidates produced by `parse_text_with_assignment` and its corresponding `AssignmentResult`, allowing review tools and automated consumers to inspect both the decision and the evidence behind it.

## Tabular header context and row assignment

```rust
pub struct TableCell {
    pub source_column: usize,
    pub value: RawValue,
    pub source_block_id: String,
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
assignment behavior. Excluded blocks are warned about but their raw values are
not embedded in the table result.

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

The envelope contains no `RawDocument`, source filename, complete location
index, or noncandidate text. Original values remain in the caller's input
document, not in CLI parse output. Assignment warnings exist within each
record's `parse.assignment.warnings`; top-level warnings cover row grouping.
Input document warnings are not propagated. Rejected-fragment accounting,
record statuses and statistics below are proposed, not fields of version `0.1`.
[#10](https://github.com/2001J/fuzzy-parser/issues/10) must add review evidence
with an explicit compatibility/migration contract.

## Parsed record — proposed

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
output validates them. In particular, safe diagnostic redaction and complete
source maps remain open work.

- Use explicit enums rather than magic strings internally.
- JSON field names must remain stable once external integrations depend on them.
- Unknown future fields should not break tolerant readers where possible.
- Errors and warnings use machine-readable codes.
- Source locations use a clearly documented indexing convention.
- Floating confidence values must have defined bounds and meaning.
- Sensitive raw input should be optionally omitted from low-privilege output modes while preserving source identifiers.

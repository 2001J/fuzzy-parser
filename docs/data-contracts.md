# Data Contracts

This document describes the intended public models. Exact Rust field names may evolve before implementation, but semantic changes must be deliberate and reflected here.

## Contract versioning

Serialized requests and responses should include a contract version once external integrations exist.

```json
{
  "contract_version": "0.1"
}
```

Parser implementation version and schema contract version are separate concerns.

## Parser input

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

The initial segmentation API uses a bounded `Confidence` score and stable reason metadata:

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

## Field candidate

```rust
pub struct FieldCandidate {
    pub candidate_type: CandidateType,
    pub raw_value: String,
    pub normalized_value: Option<serde_json::Value>,
    pub source_span: TextSpan,
    pub confidence: Confidence,
    pub reasons: Vec<Reason>,
}
```

Candidates are evidence. They are not automatically assignments.

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

`assign_candidates` matches candidate types against caller-provided fields, applies integer and length constraints, uses nearby labels as context, and serializes its result for integration surfaces. A missing value is different from an empty string. Ambiguous assignments and unassigned candidates remain observable.

## Parsed record

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

## Parse request

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

## Parse result

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

## Serialization rules

- Use explicit enums rather than magic strings internally.
- JSON field names must remain stable once external integrations depend on them.
- Unknown future fields should not break tolerant readers where possible.
- Errors and warnings use machine-readable codes.
- Source locations use a clearly documented indexing convention.
- Floating confidence values must have defined bounds and meaning.
- Sensitive raw input should be optionally omitted from low-privilege output modes while preserving source identifiers.

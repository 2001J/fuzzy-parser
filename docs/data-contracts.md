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
`0.1.0`. Errors now use `error_contract_version: "0.1"`; raw inspection has no
version field. These version axes are independent.
A unified serialized parse request is still proposed. Planning milestone names
are not any of these versions; see [release strategy](release-and-environment-strategy.md).

```json
{
  "contract_version": "0.1"
}
```

Parser implementation version and schema contract version are separate concerns.

### Executable schema migration (#12)

The #12 execution boundary preserves structural `TargetSchema` decoding and the
raw/parse-response JSON shapes. Execution deliberately rejects unsupported
options, inapplicable constraints, unknown schema properties, ambiguous enum
definitions and definitions that cannot be detected. The additive failure codes
are `schema_option_unsupported`, `schema_constraint_unsupported`,
`schema_property_unsupported`, `schema_enum_definition_ambiguous` and
`schema_enum_definition_unsupported`, under error contract `0.1`. They have no
variable safe metadata. Rust exhaustive matches on `FailureKind` and strict typed
error readers must handle these new variants. Existing codes and safe-report
rendering remain unchanged. Caller strings in errors remain diagnostics-only;
successful results and record warnings retain their existing raw-data contract.

Enum ownership corrections can change previously incorrect assignments and add
`enum_field_ambiguous` warnings. No response fields or versions are added.
Profiles relying on ignored options, ineffective constraints, unknown properties
or pooled enum assignments must migrate to the [executable capabilities below](#executable-schema).
Structural schema validation alone remains compatible and is not proof of execution support.
The [pre-change fixture](../fixtures/contracts/schema-compilation-before.json)
locks complete CLI output for contact, attendance and inventory profiles across
TXT, stdin, CSV and XLSX; tests also compare the same output with library execution.

### Error contract 0.1 and migration from unversioned errors

[#2](https://github.com/2001J/fuzzy-parser/issues/2) deliberately changes error
serialization and default human messages. Its implementation is independently
reviewed and verified locally; it is not a package release. Success JSON, schema JSON,
`ParseResponse.contract_version`, source evidence and warnings are unchanged.
The CLI keeps its outer `error`/`message` envelope, JSON stderr and exit `1`:

```json
{
  "error": {
    "error_contract_version": "0.1",
    "code": "io_error",
    "kind": "not_found"
  },
  "message": "could not read input: file not found"
}
```

[`parser-core::Failure`](../crates/parser-core/src/errors.rs) is the shared typed
boundary. `FailureKind` carries safe category metadata; `ErrorPayload` is the
wire payload above; `ErrorReport` adds the outer message. `Failure::report(mode)`
and `ParserError::report(mode)` use the same exhaustive renderer as default
`Display`. Schema causes convert to `Failure` and expose the same report method.
`ParserError` and `Failure` default `Serialize` emit the **bare safe payload**,
not the outer CLI envelope.

`ErrorReport` stores only its public typed `error` payload. Its `message()`
accessor returns a freshly rendered `String`; there is no independently mutable
`message` field. Serialization always derives the outer message from the same
payload as `Display`, including after payload changes. Deserialization
**canonicalizes**: it ignores incoming outer `message` prose (also accepting an
absent or non-string message) and never stores or re-emits it. The typed `error`
is still required and validated. Generated safe/detailed envelopes retain exact
round trips; forged or stale outer messages intentionally do not. Explicit
diagnostics inside `ErrorPayload` are preserved, not redacted by decoding.

All thirteen previous codes retain their meanings. The table lists default
metadata beyond `code` and `error_contract_version`; private context is absent:

| Code | Default metadata |
| --- | --- |
| `io_error` | `kind` |
| `invalid_utf8` | `valid_up_to` byte offset |
| `unsupported_input` | None; includes file-extension eligibility failures |
| `input_too_large` | `limit`, `actual` byte counts |
| `line_too_long` | `line` (one-based), `limit`, `actual` byte counts |
| `invalid_csv` | `record` (one-based) or explicit `null` |
| `invalid_xlsx` | None |
| `schema_io_error` | `kind`; includes invalid schema UTF-8 |
| `schema_input_error` | None; non-UTF-8 inline schema OS argument |
| `schema_parse_error` | None; invalid schema JSON |
| `schema_validation_error` | Typed `reason` |
| `schema_field_type_unsupported` | Known literal `field_type`: `text`, `person_name`, or `datetime` |
| `schema_option_unsupported`, `schema_constraint_unsupported`, `schema_property_unsupported` | None; execution capability failures |
| `schema_enum_definition_ambiguous`, `schema_enum_definition_unsupported` | None; executable enum definition failures |
| `schema_serialization_error` | Typed `cause`: `{"kind":"json"}` or `{"kind":"validation","reason":...}` |
| `output_serialization_error` (new) | Fixed `target`: `parse_result` or `raw_document` |
| `not_regular_file` (#5) | None |
| `empty_input` (#5) | None |
| `file_too_large` (#5) | Exact `u64` metadata byte counts: `limit`, `actual` |

Validation reasons are `empty_schema_version`, `unsupported_schema_version`,
`empty_field_name`, `duplicate_field_name`, `duplicate_field_label`, `empty_alias`,
`empty_enum_value`, `duplicate_enum_value`, `empty_enum_alias`, `duplicate_enum_alias`,
`invalid_integer_range`, and `invalid_length_range`. Numeric metadata is not
new validation policy. Bounded reads can report the observed `limit + 1` bytes
rather than the complete size of an unread remainder.

`IoErrorKind` now has the Rust variant `InvalidData` / JSON `invalid_data`.
Conversion from `std::io::ErrorKind::InvalidData` deliberately refines its old
`Other` mapping. Other existing kinds remain `not_found`, `permission_denied`,
`invalid_input`, and `other`. Rust exhaustive matches must handle the addition.
Schema UTF-8 failures retain `schema_io_error`, not `schema_parse_error`.

#### Explicit detailed diagnostics

`DiagnosticsMode::Safe` is the default library mode; request `Detailed` explicitly
through `error.report(DiagnosticsMode::Detailed)`. The CLI equivalent is a single
leading `--diagnostics` before the command (see [usage](integration-strategy.md#current-cli-boundary)).
Only this opt-in can add `error.diagnostics`. It contains available, typed,
allowlisted context: `path`, `source`, `field`, `value`, `alias`, `version`, or
`source_type`. Absent context stays absent. An empty byte-XLSX error path does not
become a fabricated path or filename. Duplicate field labels use `value` because
the existing cause does not distinguish a field name from an alias.

Detailed `Display` and the report's message append
` [diagnostics: {JSON-escaped context}]` to the same safe message. Control
characters are escaped, not emitted as terminal controls. Full input/schema and
opaque dependency/OS/serde prose are never copied into reports, even in this mode.
Detailed diagnostics **may still contain sensitive caller data**. They and raw
in-process cause fields / `Debug` output are not safe for public logs. Successful
source-backed results also remain sensitive; this is not success-output redaction.

#### Consumer migration and deserialization

- Stop reading default `path`, `source`, `source_type`, opaque CSV/XLSX `message`,
  or redundant schema `error.message`. Use the stable code and typed metadata;
  use the outer message for display, not as a machine discriminator. Explicit
  diagnostics are for authorized troubleshooting, not a replacement public log.
- Existing in-process `ParserError` and schema cause variants/data/return types
  remain, including #22's exact file paths and internal XLSX cause strings.
  `SchemaParseError::source()` now exposes the actual nested validation error;
  already-flattened OS/serde strings are not reconstructed as invented causes.
- `ParserError::Deserialize` intentionally still reads **legacy cause JSON**
  with its original required private fields. Its new redacted serialization
  cannot reconstruct those fields and does **not** round-trip as `ParserError`.
  Decode new output as `ErrorPayload` (or `ErrorReport` for the CLI envelope).
  The [legacy cause fixture](../fixtures/contracts/errors-legacy.json) retains
  read compatibility; it is not the new output shape.
- `ErrorPayload` round-trips supported typed payloads, with or without explicit
  diagnostics. It requires version `"0.1"` and rejects missing/unknown versions,
  unknown failure variants and unknown diagnostic keys. Unknown outer payload
  keys are ignored, not retained; this is not arbitrary-JSON round-trip support.
- Update strict JSON consumers for the version field, typed schema metadata and
  additive output-serialization code. Usage failures remain plain stderr/exit
  `2`, including non-UTF-8 `inspect --text` OS arguments. No command-routing or
  extra-argument cleanup is included.

The [pre-migration success goldens](../fixtures/contracts/cli-success-before-errors.json)
fix exact stdout for TXT/pasted/CSV/XLSX inspection, schema output and a source-backed
parse. [Testing strategy](testing-strategy.md#error-contract-regressions) describes
coverage and the serialization-failure branches tested only at the report boundary.

### File-validation additions in error contract 0.1

The local #5 slice adds `not_regular_file`, `empty_input`, and
`file_too_large` to `ParserError` and `FailureKind`, retaining error version
`0.1`. The first two have no default metadata; `file_too_large` has exact `u64`
`limit` and `actual` byte counts from metadata. Fixed messages are respectively
`input is not a regular file`, `empty input is not allowed`, and
`file exceeds the {limit}-byte limit ({actual} bytes)`. Only explicit detailed
diagnostics may include the supplied `path`. Existing payload shapes, private
legacy-cause decoding, and safe/detailed rendering rules remain unchanged.
Consumers with exhaustive Rust matches or strict typed JSON readers must add
these cases; older readers may reject new codes even though the version remains
`0.1`. This is an intentional pre-1.0 extension, not a package release.

TXT path validation deliberately rejects non-`.txt` extensions (case
insensitive), including absent/non-UTF-8 extensions, using `unsupported_input`.
The CLI's existing unknown-extension fallback consequently fails in the TXT
library instead of accepting arbitrary file names; routing itself remains #6.
Directory paths now yield `not_regular_file`, rather than a platform-dependent
I/O cause. Metadata oversize now yields `file_too_large` before decoding;
bounded-read overflow (including growth after validation) retains
`input_too_large` with its original `usize` fields and observed-byte meaning.
No metadata length is truncated to fit those legacy fields.

Zero-byte acceptance remains the explicit default; callers may reject empty
files at both metadata and actual-read checks. Whitespace alone is not empty.
Successful TXT values/metadata, byte/text/stdin inputs, existing line limits,
and other format readers are unchanged. See [file validation](file-validation.md)
for API defaults, ordering, overrides and filesystem race limits.

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

Missing files and invalid file reads preserve the existing in-process error
categories, supplied paths and cause data. Invalid byte input returns `InvalidXlsx`
(`invalid_xlsx`) with `path: ""` and the generic message
`could not read XLSX workbook`; no filename, sheet name or workbook content is
added to diagnostics. The empty string preserves the existing required string
field without inventing a filesystem path. Default serialization and Display
now use the [safe error contract](#error-contract-01-and-migration-from-unversioned-errors);
the empty internal path is absent even from explicit diagnostics.

This additive library API does not change CLI commands or JSON/package versions.
The local [#22](https://github.com/2001J/fuzzy-parser/issues/22) implementation and
file-reader parity are independently verified. Workbook/decompression/cell/output limits remain
[#17](https://github.com/2001J/fuzzy-parser/issues/17); WASM/JS execution and runtime
packaging remain separate capabilities. [Compiled plan execution](#executable-schema)
accepts canonical documents from either XLSX input API.

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

Validation accepts the field vocabulary above. Enum JSON uses an externally tagged object such as
`{"enum":{"values":[{"value":"active","aliases":["enabled"]}]}}`.
Constraints are tagged with `kind`/`value`.

The CLI preserves this validation contract for file, standard-input, and inline-text sources. Valid output is pretty-printed by default and can be emitted as one compact JSON line for pipeline consumers.

### Executable schema

`parser_schema::compile_schema(&TargetSchema) -> Result<parser_core::ParsePlan, Failure>`
validates programmatically constructed schemas and compiles supported behavior.
`parser_core::parse_document_with_plan(&RawDocument, &ParsePlan) -> ParseResponse`
reuses the existing document pipeline, detectors and assignment internals. The
plan is reusable, stores no input and has no JSON representation. Its field
representation is private; `PlanField::new` and `ParsePlan::new` are low-level
runtime constructors, not substitutes for schema validation.

JSON callers should use `parser_schema::compile_schema_json(&str)` to retain strict
execution checks. `decode_execution_schema(&str)` exposes strict decoding separately
from compilation; the CLI uses this split to preserve its existing precedence:
schema decoding/structural validation before input reading, then capability
compilation after successful extraction. Existing structural errors retain priority,
and compilation checks the first unsupported type before new option/constraint/enum
capability failures. Unknown members are rejected at modeled schema, option,
field, constraint and enum-definition objects before they can be silently lost.
Missing required JSON members still produce `schema_parse_error`; `record_name`
remains optional. `TargetSchema::from_json`, ordinary Serde decoding, and
`schema validate` remain structural and tolerate unknown members as before.
Compiling an already decoded typed schema cannot recover discarded JSON keys.
Execution also preserves Serde's accepted positional struct arrays and unit-type
objects such as `{"email":null}`; strict traversal checks nested objects in those
representations too. Normal schema serialization still emits named properties
and string unit types.

| Field type | Executable constraints |
| --- | --- |
| `integer` | `minimum_integer`, `maximum_integer` |
| `email`, `phone_number`, `date`, `enum` | `minimum_length`, `maximum_length` |
| `decimal`, `currency`, `boolean` | None |
| `text`, `person_name`, `datetime` | Execution fails with `schema_field_type_unsupported` |

All bounds are inclusive and repeated bounds are conjoined, not overwritten.
Lengths count Unicode scalars in normalized values: email text, phone digits,
ISO date text or the canonical enum value. Structural range validation remains
unchanged; later repeated bounds can leave no eligible values. Rejected candidates
remain unassigned and a required field without an eligible value warns.
Inapplicable combinations fail with `schema_constraint_unsupported`.

`required` controls missing-field warnings; it does not make a parse fatal.
`multiple=true` retains eligible occurrences, preferring matching header columns;
`false` selects one using existing context/confidence and warns about multiple
candidates. Field names and aliases supply existing label/header context.
`record_name` is echoed only. `allow_unknown_fields=true` preserves all unused
and unassigned evidence; `false` fails with `schema_option_unsupported` until a
strict data policy exists. This option concerns input evidence, not unknown JSON
schema members. Locale, country hints, explicit columns, uniqueness and runtime
limits are not executable schema properties. Low-level `AssignmentField.unique`
still only warns when more than one candidate was selected; it is not deduplication
or database uniqueness, and compilation sets it to false.

Enum definitions belong to individual fields. Matching remains ASCII case
insensitive on whitespace-separated tokens with edge `. , ; : ( ) [ ]` trimming.
Each lexical definition must be detectable as such a token. A multiword or
otherwise undetectable canonical value is allowed when reachable through supported
short aliases, and is emitted verbatim; aliases must themselves be detectable.
There is no phrase detection. Unsupported definitions fail explicitly rather than
being omitted. An empty `values` array remains valid and matches nothing.
Within-field lexical collisions fail compilation (existing structural collision
errors retain precedence).

An occurrence is eligible only for fields whose own raw canonical/alias definition
matched it. Shared canonical strings do not share aliases. For competing owners,
a uniquely best existing header match, then nearby label match, can resolve
ownership. Ties leave all hypotheses unassigned with `enum_field_ambiguous`;
constraints and schema order cannot break them. One enum occurrence is never
assigned to two enum fields. All canonical alternatives remain detected evidence;
unchosen alternatives remain unassigned even when context selects an owner.
Other candidate types keep their independent detections and assignment behavior.

Existing `parse_document_with_assignment`, `parse_text_with_assignment`, table and
standalone assignment APIs retain their signatures and legacy caller-supplied
global enum semantics. Use the compiled plan for field-scoped schema execution.
Both routes share core internals; neither composes normalization/segmentation yet.

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

`parse_document_with_plan` and `parse_document_with_assignment` choose table mode
when any blocks have row provenance, otherwise one text record per raw block.
Neither runs the separate
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
output validates them. Arbitrary normalization source maps, optional redaction
of successful raw results and adapter-level original-file fidelity remain open work.

- Use explicit enums rather than magic strings internally.
- JSON field names must remain stable once external integrations depend on them.
- Unknown future fields should not break tolerant readers where possible.
- Errors and warnings use machine-readable codes.
- Source locations use a clearly documented indexing convention.
- Floating confidence values must have defined bounds and meaning.
- Sensitive raw input should be optionally omitted from low-privilege output modes while preserving source identifiers.

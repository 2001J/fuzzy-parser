use serde::{Deserialize, Serialize};
use std::{fmt, io};

pub const CONTRACT_VERSION: &str = "0.1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    Text,
    Stdin,
    Txt,
    Csv,
    Xlsx,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", content = "value")]
pub enum RawValue {
    Text(String),
    Integer(i64),
    Decimal(f64),
    Boolean(bool),
    DateTime(f64),
    DateTimeText(String),
    Duration(String),
    Error(String),
    Null,
}

impl RawValue {
    pub fn text(value: impl Into<String>) -> Self {
        Self::Text(value.into())
    }

    pub fn to_text(&self) -> String {
        match self {
            Self::Text(value) => value.clone(),
            Self::Integer(value) => value.to_string(),
            Self::Decimal(value) => value.to_string(),
            Self::Boolean(value) => value.to_string(),
            Self::DateTime(value) => value.to_string(),
            Self::DateTimeText(value) => value.clone(),
            Self::Duration(value) => value.clone(),
            Self::Error(value) => value.clone(),
            Self::Null => String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceMetadata {
    pub source_type: SourceType,
    pub file_name: Option<String>,
    pub mime_type: Option<String>,
    pub size_bytes: Option<u64>,
    pub delimiter: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SourceLocation {
    pub line: Option<usize>,
    pub row: Option<usize>,
    pub column: Option<usize>,
    pub sheet: Option<String>,
    pub byte_start: Option<usize>,
    pub byte_end: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RawBlock {
    pub id: String,
    pub value: RawValue,
    pub location: SourceLocation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NormalizationOptions {
    pub normalize_line_endings: bool,
    pub trim_whitespace: bool,
    pub collapse_whitespace: bool,
    pub normalize_punctuation: bool,
    pub mark_noise: bool,
}

impl Default for NormalizationOptions {
    fn default() -> Self {
        Self {
            normalize_line_endings: true,
            trim_whitespace: true,
            collapse_whitespace: true,
            normalize_punctuation: true,
            mark_noise: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NormalizedBlock {
    pub source_block_id: String,
    pub original: RawValue,
    pub normalized_text: String,
    pub transformations: Vec<Transformation>,
}

pub type Confidence = f64;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Reason {
    pub code: String,
    pub message: String,
}

impl Reason {
    fn new(code: &'static str, message: &'static str) -> Self {
        Self {
            code: code.to_owned(),
            message: message.to_owned(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SegmentationStrategy {
    OneBlockPerRecord,
    OneRowPerRecord,
    JoinIndentedContinuations,
    SplitRepeatedIdentifiers,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SegmentationOptions {
    pub strategy: SegmentationStrategy,
    pub join_separator: String,
}

impl Default for SegmentationOptions {
    fn default() -> Self {
        Self {
            strategy: SegmentationStrategy::OneBlockPerRecord,
            join_separator: "\n".to_owned(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecordCandidate {
    pub id: String,
    pub source_block_ids: Vec<String>,
    pub text: String,
    pub confidence: Confidence,
    pub reasons: Vec<Reason>,
    pub warnings: Vec<ParserWarning>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TextSpan {
    pub byte_start: usize,
    pub byte_end: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CandidateType {
    Email,
    Integer,
    Decimal,
    PhoneNumber,
    Boolean,
    Date,
    Currency,
    Enum,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FieldCandidate {
    pub candidate_type: CandidateType,
    pub raw_value: String,
    pub normalized_value: Option<serde_json::Value>,
    pub source_span: TextSpan,
    #[serde(default)]
    pub source_column: Option<usize>,
    pub confidence: Confidence,
    pub reasons: Vec<Reason>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AssignmentConstraint {
    MinimumInteger(i64),
    MaximumInteger(i64),
    MinimumLength(usize),
    MaximumLength(usize),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssignedField {
    pub name: String,
    pub candidates: Vec<FieldCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssignmentResult {
    pub fields: Vec<AssignedField>,
    pub unassigned_candidates: Vec<FieldCandidate>,
    pub warnings: Vec<ParserWarning>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TextParseResult {
    pub candidates: Vec<FieldCandidate>,
    pub assignment: AssignmentResult,
}

pub fn parse_text_with_assignment(
    text: &str,
    fields: &[AssignmentField],
    enum_definitions: &[(String, Vec<String>)],
) -> TextParseResult {
    let candidates = collect_field_candidates(text, enum_definitions);
    let assignment = assign_candidates(text, &candidates, fields);
    TextParseResult {
        candidates,
        assignment,
    }
}

fn collect_field_candidates(
    text: &str,
    enum_definitions: &[(String, Vec<String>)],
) -> Vec<FieldCandidate> {
    let mut candidates = Vec::new();
    candidates.extend(detect_email_candidates(text));
    candidates.extend(detect_integer_candidates(text));
    candidates.extend(detect_decimal_candidates(text));
    candidates.extend(detect_phone_candidates(text));
    candidates.extend(detect_boolean_candidates(text));
    candidates.extend(detect_date_candidates(text));
    candidates.extend(detect_currency_candidates(text));
    candidates.extend(detect_enum_candidates(text, enum_definitions));
    candidates
}

/// One source cell inside a tabular row group.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TableCell {
    /// One-based column number matching `SourceLocation.column`.
    pub source_column: usize,
    pub value: RawValue,
    pub source_block_id: String,
}

/// All cells of one sheet row, ordered by column.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TableRowGroup {
    pub sheet: Option<String>,
    pub source_row: usize,
    pub cells: Vec<TableCell>,
    pub source_block_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DocumentRows {
    pub rows: Vec<TableRowGroup>,
    pub warnings: Vec<ParserWarning>,
}

/// Groups blocks that carry row provenance into per-row cell groups.
///
/// Blocks without row metadata are never dropped: they are reported as
/// structured warnings so callers can see what the grouping excluded.
pub fn group_document_rows(document: &RawDocument) -> DocumentRows {
    let mut rows: Vec<TableRowGroup> = Vec::new();
    let mut warnings = Vec::new();

    for block in &document.blocks {
        let Some(source_row) = block.location.row else {
            warnings.push(ParserWarning {
                code: "row_provenance_missing".to_owned(),
                message: format!(
                    "block {} has no row metadata, so it was excluded from tabular grouping",
                    block.id
                ),
                location: Some(block.location.clone()),
            });
            continue;
        };

        if let Some(group) = rows
            .iter_mut()
            .find(|group| group.sheet == block.location.sheet && group.source_row == source_row)
        {
            group.cells.push(TableCell {
                source_column: block.location.column.unwrap_or(0),
                value: block.value.clone(),
                source_block_id: block.id.clone(),
            });
            group.source_block_ids.push(block.id.clone());
        } else {
            rows.push(TableRowGroup {
                sheet: block.location.sheet.clone(),
                source_row,
                cells: vec![TableCell {
                    source_column: block.location.column.unwrap_or(0),
                    value: block.value.clone(),
                    source_block_id: block.id.clone(),
                }],
                source_block_ids: vec![block.id.clone()],
            });
        }
    }

    for group in &mut rows {
        group.cells.sort_by_key(|cell| cell.source_column);
    }
    rows.sort_by(|left, right| {
        left.sheet
            .cmp(&right.sheet)
            .then(left.source_row.cmp(&right.source_row))
    });

    DocumentRows { rows, warnings }
}

/// Header labels for one sheet: `(one-based column number, label)` pairs.
///
/// Labels are derived (trimmed) text; the raw values stay in the document.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TableHeaderContext {
    pub sheet: Option<String>,
    pub source_row: usize,
    pub labels: Vec<(usize, String)>,
    pub source_block_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum HeaderExtraction {
    Detected { headers: Box<TableHeaderContext> },
    NotDetected { code: String, message: String },
}

impl HeaderExtraction {
    fn not_detected(code: &'static str, message: &'static str) -> Self {
        Self::NotDetected {
            code: code.to_owned(),
            message: message.to_owned(),
        }
    }

    pub fn context(&self) -> Option<&TableHeaderContext> {
        match self {
            Self::Detected { headers } => Some(headers),
            Self::NotDetected { .. } => None,
        }
    }
}

/// Conservatively decides whether the first row of a sheet is a header row.
///
/// A row is only treated as a header when the sheet has at least two rows and
/// every cell of the first row is non-empty plain text without strongly typed
/// values. Anything else is reported as not detected with a reason.
pub fn detect_table_headers(rows: &[TableRowGroup]) -> HeaderExtraction {
    let Some(header_row) = rows.first() else {
        return HeaderExtraction::not_detected(
            "header_not_detected_no_rows",
            "there were no rows available to inspect for a header",
        );
    };
    if rows.len() < 2 {
        return HeaderExtraction::not_detected(
            "header_not_detected_single_row",
            "a single row cannot be safely distinguished from a header row",
        );
    }

    let mut labels = Vec::new();
    for cell in &header_row.cells {
        let RawValue::Text(value) = &cell.value else {
            return HeaderExtraction::not_detected(
                "header_not_detected_typed_cell",
                "the first row contains a typed cell, so it was not treated as a header",
            );
        };
        let label = collapse_whitespace(value.trim());
        if label.is_empty() {
            return HeaderExtraction::not_detected(
                "header_not_detected_empty_cell",
                "the first row contains an empty cell, so it was not treated as a header",
            );
        }
        labels.push((cell.source_column, label));
    }

    if labels.len() < 2 {
        return HeaderExtraction::not_detected(
            "header_not_detected_too_few_cells",
            "the first row has fewer than two labeled cells, so it was not treated as a header",
        );
    }

    let header_text = labels
        .iter()
        .map(|(_, label)| label.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    if !collect_field_candidates(&header_text, &[]).is_empty() {
        return HeaderExtraction::not_detected(
            "header_not_detected_strong_values",
            "the first row contains strongly typed values, so it was not treated as a header",
        );
    }

    HeaderExtraction::Detected {
        headers: Box::new(TableHeaderContext {
            sheet: header_row.sheet.clone(),
            source_row: header_row.source_row,
            labels,
            source_block_ids: header_row.source_block_ids.clone(),
        }),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TableRowParseResult {
    pub source_row: usize,
    pub source_block_ids: Vec<String>,
    pub parse: TextParseResult,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SheetTableResult {
    pub sheet: Option<String>,
    pub header: HeaderExtraction,
    pub records: Vec<TableRowParseResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TableParseResult {
    pub sheets: Vec<SheetTableResult>,
    pub warnings: Vec<ParserWarning>,
}

/// Parses a tabular document row by row using header-derived assignment context.
///
/// The pipeline composes existing stages: canonical blocks are grouped into
/// rows, each sheet's first row is conservatively evaluated as a header, and
/// every remaining row is parsed with candidate detection plus schema-compatible
/// assignment where header labels act as column-level evidence. Raw values stay
/// in the document; unassigned candidates remain observable.
pub fn parse_document_rows_with_assignment(
    document: &RawDocument,
    fields: &[AssignmentField],
    enum_definitions: &[(String, Vec<String>)],
) -> TableParseResult {
    let grouped = group_document_rows(document);

    let mut buckets: Vec<(Option<String>, Vec<TableRowGroup>)> = Vec::new();
    for row in grouped.rows {
        match buckets.iter_mut().find(|(sheet, _)| *sheet == row.sheet) {
            Some((_, bucket)) => bucket.push(row),
            None => buckets.push((row.sheet.clone(), vec![row])),
        }
    }

    let sheets = buckets
        .into_iter()
        .map(|(sheet, rows)| {
            let header = detect_table_headers(&rows);
            let data_rows = rows.iter().filter(|row| {
                header
                    .context()
                    .is_none_or(|headers| row.source_row != headers.source_row)
            });
            let records = data_rows
                .map(|row| parse_row_group(row, header.context(), fields, enum_definitions))
                .collect();
            SheetTableResult {
                sheet,
                header,
                records,
            }
        })
        .collect();

    TableParseResult {
        sheets,
        warnings: grouped.warnings,
    }
}

fn parse_row_group(
    row: &TableRowGroup,
    header: Option<&TableHeaderContext>,
    fields: &[AssignmentField],
    enum_definitions: &[(String, Vec<String>)],
) -> TableRowParseResult {
    let mut row_text = String::new();
    let mut candidates = Vec::new();

    for cell in &row.cells {
        let cell_text = cell.value.to_text();
        let trimmed = cell_text.trim();
        if trimmed.is_empty() {
            continue;
        }
        if !row_text.is_empty() {
            row_text.push(' ');
        }
        let offset = row_text.len();
        row_text.push_str(trimmed);

        for mut candidate in collect_field_candidates(trimmed, enum_definitions) {
            candidate.source_column = Some(cell.source_column);
            candidate.source_span.byte_start += offset;
            candidate.source_span.byte_end += offset;
            candidates.push(candidate);
        }
    }

    let assignment = assign_candidates_inner(&row_text, &candidates, fields, header);
    TableRowParseResult {
        source_row: row.source_row,
        source_block_ids: row.source_block_ids.clone(),
        parse: TextParseResult {
            candidates,
            assignment,
        },
    }
}

pub fn assign_candidates(
    text: &str,
    candidates: &[FieldCandidate],
    fields: &[AssignmentField],
) -> AssignmentResult {
    assign_candidates_inner(text, candidates, fields, None)
}

pub fn assign_candidates_with_header_context(
    text: &str,
    candidates: &[FieldCandidate],
    fields: &[AssignmentField],
    header: Option<&TableHeaderContext>,
) -> AssignmentResult {
    assign_candidates_inner(text, candidates, fields, header)
}

fn assign_candidates_inner(
    text: &str,
    candidates: &[FieldCandidate],
    fields: &[AssignmentField],
    header: Option<&TableHeaderContext>,
) -> AssignmentResult {
    let mut assigned = vec![false; candidates.len()];
    let mut result_fields = Vec::new();
    let mut warnings = Vec::new();

    for field in fields {
        let matching_indices = candidates
            .iter()
            .enumerate()
            .filter(|(index, candidate)| {
                !assigned[*index]
                    && candidate.candidate_type == field.candidate_type
                    && candidate_satisfies_constraints(candidate, field)
            })
            .map(|(index, _)| index)
            .collect::<Vec<_>>();

        if matching_indices.is_empty() {
            if field.required {
                warnings.push(ParserWarning {
                    code: "required_field_missing".to_owned(),
                    message: format!("required field {} has no compatible candidate", field.name),
                    location: None,
                });
            }
            continue;
        }

        // When a header context is available, prefer candidates whose column
        // carries a matching header label over equally type-compatible ones.
        let eligible_indices =
            narrow_by_header_context(&matching_indices, candidates, field, header);

        let selected_indices = if field.multiple {
            eligible_indices
        } else {
            if eligible_indices.len() > 1 {
                warnings.push(ParserWarning {
                    code: "multiple_candidates_ambiguous".to_owned(),
                    message: format!(
                        "field {} has multiple compatible candidates; the highest-confidence candidate was selected",
                        field.name
                    ),
                    location: None,
                });
            }
            vec![select_highest_confidence(
                text,
                candidates,
                &eligible_indices,
                field,
                header,
            )]
        };

        let selected: Vec<FieldCandidate> = selected_indices
            .into_iter()
            .map(|index| {
                assigned[index] = true;
                let mut candidate = candidates[index].clone();
                if candidate_matches_field_header(&candidate, field, header) {
                    candidate.reasons.push(Reason::new(
                        "header_label_match",
                        "the candidate's source column had a header label matching the field",
                    ));
                }
                candidate
            })
            .collect();
        if field.unique && selected.len() > 1 {
            warnings.push(ParserWarning {
                code: "unique_field_multiple_values".to_owned(),
                message: format!("unique field {} received multiple candidates", field.name),
                location: None,
            });
        }
        result_fields.push(AssignedField {
            name: field.name.clone(),
            candidates: selected,
        });
    }

    let unassigned_candidates = candidates
        .iter()
        .enumerate()
        .filter(|(index, _)| !assigned[*index])
        .map(|(_, candidate)| candidate.clone())
        .collect();

    AssignmentResult {
        fields: result_fields,
        unassigned_candidates,
        warnings,
    }
}

fn candidate_satisfies_constraints(candidate: &FieldCandidate, field: &AssignmentField) -> bool {
    field.constraints.iter().all(|constraint| match constraint {
        AssignmentConstraint::MinimumInteger(minimum) => candidate
            .normalized_value
            .as_ref()
            .and_then(serde_json::Value::as_i64)
            .is_some_and(|value| value >= *minimum),
        AssignmentConstraint::MaximumInteger(maximum) => candidate
            .normalized_value
            .as_ref()
            .and_then(serde_json::Value::as_i64)
            .is_some_and(|value| value <= *maximum),
        AssignmentConstraint::MinimumLength(minimum) => candidate
            .normalized_value
            .as_ref()
            .and_then(serde_json::Value::as_str)
            .is_some_and(|value| value.chars().count() >= *minimum),
        AssignmentConstraint::MaximumLength(maximum) => candidate
            .normalized_value
            .as_ref()
            .and_then(serde_json::Value::as_str)
            .is_some_and(|value| value.chars().count() <= *maximum),
    })
}

fn narrow_by_header_context(
    matching_indices: &[usize],
    candidates: &[FieldCandidate],
    field: &AssignmentField,
    header: Option<&TableHeaderContext>,
) -> Vec<usize> {
    let Some(header) = header else {
        return matching_indices.to_vec();
    };
    let matched = matching_indices
        .iter()
        .copied()
        .filter(|index| candidate_matches_field_header(&candidates[*index], field, Some(header)))
        .collect::<Vec<_>>();
    if matched.is_empty() || matched.len() == matching_indices.len() {
        matching_indices.to_vec()
    } else {
        matched
    }
}

fn header_label_for_column(
    header: Option<&TableHeaderContext>,
    column: Option<usize>,
) -> Option<&str> {
    let header = header?;
    let column = column?;
    header
        .labels
        .iter()
        .find(|(label_column, _)| *label_column == column)
        .map(|(_, label)| label.as_str())
}

fn candidate_matches_field_header(
    candidate: &FieldCandidate,
    field: &AssignmentField,
    header: Option<&TableHeaderContext>,
) -> bool {
    let Some(label) = header_label_for_column(header, candidate.source_column) else {
        return false;
    };
    let label = label.to_ascii_lowercase();
    std::iter::once(field.name.as_str())
        .chain(field.aliases.iter().map(String::as_str))
        .any(|name| name.to_ascii_lowercase() == label)
}

fn select_highest_confidence(
    text: &str,
    candidates: &[FieldCandidate],
    indices: &[usize],
    field: &AssignmentField,
    header: Option<&TableHeaderContext>,
) -> usize {
    indices
        .iter()
        .copied()
        .max_by(|left, right| {
            candidate_score(text, &candidates[*left], field, header)
                .partial_cmp(&candidate_score(text, &candidates[*right], field, header))
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| right.cmp(left))
        })
        .expect("assignment requires at least one matching candidate")
}

fn candidate_score(
    text: &str,
    candidate: &FieldCandidate,
    field: &AssignmentField,
    header: Option<&TableHeaderContext>,
) -> (bool, bool, bool, f64) {
    let has_header_context = candidate_matches_field_header(candidate, field, header);
    let context_start = candidate.source_span.byte_start.saturating_sub(40);
    let context = text[context_start..candidate.source_span.byte_start].to_ascii_lowercase();
    let labels =
        std::iter::once(field.name.as_str()).chain(field.aliases.iter().map(String::as_str));
    let has_label_context = labels
        .map(|label| format!("{}:", label.to_ascii_lowercase()))
        .any(|label| context.contains(&label));
    let has_column_context = field.expected_column.is_some_and(|column| {
        candidate
            .source_column
            .is_some_and(|candidate_column| candidate_column == column)
    });
    (
        has_header_context,
        has_label_context,
        has_column_context,
        candidate.confidence,
    )
}

pub fn detect_email_candidates(text: &str) -> Vec<FieldCandidate> {
    text.split_whitespace()
        .scan(0, |search_start, token| {
            let start = text[*search_start..].find(token)? + *search_start;
            *search_start = start + token.len();
            Some((start, token))
        })
        .filter_map(|(token_start, token)| {
            let value = token.trim_matches(|character: char| {
                matches!(
                    character,
                    '.' | ',' | ';' | ':' | '(' | ')' | '[' | ']' | '<' | '>'
                )
            });
            if !is_email(value) {
                return None;
            }
            let value_offset = token.find(value)?;
            let byte_start = token_start + value_offset;
            let byte_end = byte_start + value.len();
            Some(FieldCandidate {
                candidate_type: CandidateType::Email,
                raw_value: value.to_owned(),
                normalized_value: Some(serde_json::Value::String(value.to_ascii_lowercase())),
                source_span: TextSpan {
                    byte_start,
                    byte_end,
                },
                source_column: None,
                confidence: 0.98,
                reasons: vec![Reason::new(
                    "email_pattern_match",
                    "the value matches a conservative email pattern",
                )],
            })
        })
        .collect()
}

fn is_email(value: &str) -> bool {
    let Some((local, domain)) = value.split_once('@') else {
        return false;
    };
    !local.is_empty()
        && !domain.is_empty()
        && domain.contains('.')
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric()
                || matches!(character, '.' | '_' | '%' | '+' | '-' | '@')
        })
}

pub fn detect_integer_candidates(text: &str) -> Vec<FieldCandidate> {
    text.split_whitespace()
        .scan(0, |search_start, token| {
            let start = text[*search_start..].find(token)? + *search_start;
            *search_start = start + token.len();
            Some((start, token))
        })
        .filter_map(|(token_start, token)| {
            let value = token.trim_matches(|character: char| {
                matches!(character, '.' | ',' | ';' | ':' | '(' | ')' | '[' | ']')
            });
            let parsed = value.parse::<i64>().ok()?;
            let value_offset = token.find(value)?;
            let byte_start = token_start + value_offset;
            let byte_end = byte_start + value.len();
            Some(FieldCandidate {
                candidate_type: CandidateType::Integer,
                raw_value: value.to_owned(),
                normalized_value: Some(serde_json::Value::Number(parsed.into())),
                source_span: TextSpan {
                    byte_start,
                    byte_end,
                },
                source_column: None,
                confidence: 0.96,
                reasons: vec![Reason::new(
                    "integer_pattern_match",
                    "the value is a complete signed integer token",
                )],
            })
        })
        .collect()
}

pub fn detect_decimal_candidates(text: &str) -> Vec<FieldCandidate> {
    text.split_whitespace()
        .scan(0, |search_start, token| {
            let start = text[*search_start..].find(token)? + *search_start;
            *search_start = start + token.len();
            Some((start, token))
        })
        .filter_map(|(token_start, token)| {
            let value = token.trim_matches(|character: char| {
                matches!(character, ',' | ';' | ':' | '(' | ')' | '[' | ']')
            });
            if !value.contains('.') || value.parse::<f64>().is_err() || value.parse::<i64>().is_ok()
            {
                return None;
            }
            let value_offset = token.find(value)?;
            let byte_start = token_start + value_offset;
            let byte_end = byte_start + value.len();
            Some(FieldCandidate {
                candidate_type: CandidateType::Decimal,
                raw_value: value.to_owned(),
                normalized_value: Some(serde_json::json!(value.parse::<f64>().ok()?)),
                source_span: TextSpan {
                    byte_start,
                    byte_end,
                },
                source_column: None,
                confidence: 0.94,
                reasons: vec![Reason::new(
                    "decimal_pattern_match",
                    "the value is a complete decimal token",
                )],
            })
        })
        .collect()
}

pub fn detect_phone_candidates(text: &str) -> Vec<FieldCandidate> {
    text.split_whitespace()
        .scan(0, |search_start, token| {
            let start = text[*search_start..].find(token)? + *search_start;
            *search_start = start + token.len();
            Some((start, token))
        })
        .filter_map(|(token_start, token)| {
            let value = token.trim_matches(|character: char| {
                matches!(character, '.' | ',' | ';' | ':' | '(' | ')' | '[' | ']')
            });
            let digits = value
                .chars()
                .filter(char::is_ascii_digit)
                .collect::<String>();
            if !(7..=15).contains(&digits.len())
                || !value.chars().all(|character| {
                    character.is_ascii_digit() || matches!(character, '+' | '-' | '(' | ')' | '.')
                })
            {
                return None;
            }
            let value_offset = token.find(value)?;
            let byte_start = token_start + value_offset;
            let byte_end = byte_start + value.len();
            Some(FieldCandidate {
                candidate_type: CandidateType::PhoneNumber,
                raw_value: value.to_owned(),
                normalized_value: Some(serde_json::Value::String(digits)),
                source_span: TextSpan {
                    byte_start,
                    byte_end,
                },
                source_column: None,
                confidence: 0.88,
                reasons: vec![Reason::new(
                    "phone_pattern_match",
                    "the value contains a plausible number of phone digits and valid separators",
                )],
            })
        })
        .collect()
}

pub fn detect_boolean_candidates(text: &str) -> Vec<FieldCandidate> {
    text.split_whitespace()
        .scan(0, |search_start, token| {
            let start = text[*search_start..].find(token)? + *search_start;
            *search_start = start + token.len();
            Some((start, token))
        })
        .filter_map(|(token_start, token)| {
            let value = token.trim_matches(|character: char| {
                matches!(character, '.' | ',' | ';' | ':' | '(' | ')' | '[' | ']')
            });
            let normalized = match value.to_ascii_lowercase().as_str() {
                "true" | "yes" | "on" => true,
                "false" | "no" | "off" => false,
                _ => return None,
            };
            let value_offset = token.find(value)?;
            let byte_start = token_start + value_offset;
            let byte_end = byte_start + value.len();
            Some(FieldCandidate {
                candidate_type: CandidateType::Boolean,
                raw_value: value.to_owned(),
                normalized_value: Some(serde_json::Value::Bool(normalized)),
                source_span: TextSpan {
                    byte_start,
                    byte_end,
                },
                source_column: None,
                confidence: 0.93,
                reasons: vec![Reason::new(
                    "boolean_alias_match",
                    "the value matches a configured generic boolean alias",
                )],
            })
        })
        .collect()
}

pub fn detect_date_candidates(text: &str) -> Vec<FieldCandidate> {
    text.split_whitespace()
        .scan(0, |search_start, token| {
            let start = text[*search_start..].find(token)? + *search_start;
            *search_start = start + token.len();
            Some((start, token))
        })
        .filter_map(|(token_start, token)| {
            let value = token.trim_matches(|character: char| {
                matches!(character, ',' | ';' | ':' | '(' | ')' | '[' | ']')
            });
            let (year, month, day, separator) = parse_date(value)?;
            let value_offset = token.find(value)?;
            let byte_start = token_start + value_offset;
            let byte_end = byte_start + value.len();
            Some(FieldCandidate {
                candidate_type: CandidateType::Date,
                raw_value: value.to_owned(),
                normalized_value: Some(serde_json::json!(format!("{year:04}-{month:02}-{day:02}"))),
                source_span: TextSpan {
                    byte_start,
                    byte_end,
                },
                source_column: None,
                confidence: if separator == '-' { 0.96 } else { 0.91 },
                reasons: vec![Reason::new(
                    "date_pattern_match",
                    "the value matches a validated year-month-day pattern",
                )],
            })
        })
        .collect()
}

pub fn detect_currency_candidates(text: &str) -> Vec<FieldCandidate> {
    text.split_whitespace()
        .scan(0, |search_start, token| {
            let start = text[*search_start..].find(token)? + *search_start;
            *search_start = start + token.len();
            Some((start, token))
        })
        .filter_map(|(token_start, token)| {
            let value = token.trim_matches(|character: char| {
                matches!(character, ',' | ';' | ':' | '(' | ')' | '[' | ']')
            });
            let numeric = value.trim_start_matches(['$', '€', '£', '¥']);
            if numeric == value || numeric.parse::<f64>().is_err() {
                return None;
            }
            let amount = numeric.parse::<f64>().ok()?;
            let value_offset = token.find(value)?;
            let byte_start = token_start + value_offset;
            let byte_end = byte_start + value.len();
            Some(FieldCandidate {
                candidate_type: CandidateType::Currency,
                raw_value: value.to_owned(),
                normalized_value: Some(serde_json::json!(amount)),
                source_span: TextSpan {
                    byte_start,
                    byte_end,
                },
                source_column: None,
                confidence: 0.92,
                reasons: vec![Reason::new(
                    "currency_symbol_match",
                    "the value has a recognized currency symbol and numeric amount",
                )],
            })
        })
        .collect()
}

pub fn detect_enum_candidates(
    text: &str,
    definitions: &[(String, Vec<String>)],
) -> Vec<FieldCandidate> {
    text.split_whitespace()
        .scan(0, |search_start, token| {
            let start = text[*search_start..].find(token)? + *search_start;
            *search_start = start + token.len();
            Some((start, token))
        })
        .filter_map(|(token_start, token)| {
            let value = token.trim_matches(|character: char| {
                matches!(character, '.' | ',' | ';' | ':' | '(' | ')' | '[' | ']')
            });
            let definition = definitions.iter().find(|(canonical, aliases)| {
                canonical.eq_ignore_ascii_case(value)
                    || aliases
                        .iter()
                        .any(|alias| alias.eq_ignore_ascii_case(value))
            })?;
            let value_offset = token.find(value)?;
            let byte_start = token_start + value_offset;
            let byte_end = byte_start + value.len();
            Some(FieldCandidate {
                candidate_type: CandidateType::Enum,
                raw_value: value.to_owned(),
                normalized_value: Some(serde_json::Value::String(definition.0.clone())),
                source_span: TextSpan {
                    byte_start,
                    byte_end,
                },
                source_column: None,
                confidence: 0.9,
                reasons: vec![Reason::new(
                    "enum_alias_match",
                    "the value matches a caller-provided enum value or alias",
                )],
            })
        })
        .collect()
}

fn parse_date(value: &str) -> Option<(u32, u32, u32, char)> {
    let separator = if value.contains('-') { '-' } else { '/' };
    let parts = value.split(separator).collect::<Vec<_>>();
    if parts.len() != 3 || parts.iter().any(|part| part.len() != 2 && part.len() != 4) {
        return None;
    }
    if parts[0].len() != 4 || parts[1].len() != 2 || parts[2].len() != 2 {
        return None;
    }
    let year = parts[0].parse::<u32>().ok()?;
    let month = parts[1].parse::<u32>().ok()?;
    let day = parts[2].parse::<u32>().ok()?;
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year.is_multiple_of(400) || (year.is_multiple_of(4) && !year.is_multiple_of(100)) => {
            29
        }
        2 => 28,
        _ => return None,
    };
    (day > 0 && day <= days_in_month).then_some((year, month, day, separator))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ParserWarning {
    pub code: String,
    pub message: String,
    pub location: Option<SourceLocation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RawDocument {
    pub id: String,
    pub source: SourceMetadata,
    pub blocks: Vec<RawBlock>,
    pub warnings: Vec<ParserWarning>,
}

impl RawDocument {
    pub fn new(id: impl Into<String>, source: SourceMetadata, blocks: Vec<RawBlock>) -> Self {
        Self {
            id: id.into(),
            source,
            blocks,
            warnings: Vec::new(),
        }
    }
}

pub fn normalize_block(block: &RawBlock) -> NormalizedBlock {
    normalize_block_with_options(block, &NormalizationOptions::default())
}

pub fn normalize_block_with_options(
    block: &RawBlock,
    options: &NormalizationOptions,
) -> NormalizedBlock {
    let original = block.value.clone();
    let mut transformations = Vec::new();
    let normalized_text = normalize_text(&original.to_text(), options, &mut transformations);

    NormalizedBlock {
        source_block_id: block.id.clone(),
        original,
        normalized_text,
        transformations,
    }
}

pub fn normalize_document(document: &RawDocument) -> Vec<NormalizedBlock> {
    normalize_document_with_options(document, &NormalizationOptions::default())
}

pub fn normalize_document_with_options(
    document: &RawDocument,
    options: &NormalizationOptions,
) -> Vec<NormalizedBlock> {
    document
        .blocks
        .iter()
        .map(|block| normalize_block_with_options(block, options))
        .collect()
}

fn normalize_text(
    input: &str,
    options: &NormalizationOptions,
    transformations: &mut Vec<Transformation>,
) -> String {
    let mut value = input.to_owned();

    if options.normalize_line_endings {
        let normalized = value.replace("\r\n", "\n").replace('\r', "\n");
        if normalized != value {
            transformations.push(Transformation::LineEndingsNormalized);
            value = normalized;
        }
    }

    if options.normalize_punctuation {
        let mut dash_changed = false;
        let mut quote_changed = false;
        let normalized: String = value
            .chars()
            .map(|character| match character {
                '\u{2010}' | '\u{2011}' | '\u{2012}' | '\u{2013}' | '\u{2014}' | '\u{2212}' => {
                    dash_changed = true;
                    '-'
                }
                '\u{2018}' | '\u{2019}' => {
                    quote_changed = true;
                    '\''
                }
                '\u{201c}' | '\u{201d}' => {
                    quote_changed = true;
                    '"'
                }
                _ => character,
            })
            .collect();
        if dash_changed {
            transformations.push(Transformation::DashesNormalized);
        }
        if quote_changed {
            transformations.push(Transformation::QuotesNormalized);
        }
        if normalized != value {
            value = normalized;
        }
    }

    if options.trim_whitespace {
        let normalized = value.trim().to_owned();
        if normalized != value {
            transformations.push(Transformation::WhitespaceTrimmed);
            value = normalized;
        }
    }

    if options.collapse_whitespace {
        let normalized = collapse_whitespace(&value);
        if normalized != value {
            transformations.push(Transformation::WhitespaceCollapsed);
            value = normalized;
        }
    }

    if options.mark_noise {
        mark_noise(&value, transformations);
    }

    value
}

fn collapse_whitespace(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut in_whitespace = false;

    for character in input.chars() {
        if character.is_whitespace() {
            if !in_whitespace {
                output.push(' ');
            }
            in_whitespace = true;
        } else {
            output.push(character);
            in_whitespace = false;
        }
    }

    output
}

fn mark_noise(value: &str, transformations: &mut Vec<Transformation>) {
    if has_list_marker(value) {
        transformations.push(Transformation::ListMarkerDetected);
    }
    if has_timestamp_prefix(value) {
        transformations.push(Transformation::TimestampPrefixDetected);
    }
    if has_sender_prefix(value) {
        transformations.push(Transformation::SenderPrefixDetected);
    }
    if value.starts_with('#') || (value.ends_with(':') && !value.contains(' ')) {
        transformations.push(Transformation::HeadingDetected);
    }
}

fn has_list_marker(value: &str) -> bool {
    let Some(first) = value.chars().next() else {
        return false;
    };
    if matches!(first, '-' | '*' | '•') {
        return value.chars().nth(1).is_some_and(char::is_whitespace);
    }

    let marker_end = value
        .char_indices()
        .take_while(|(_, character)| character.is_ascii_digit())
        .map(|(index, character)| index + character.len_utf8())
        .last();
    marker_end.is_some_and(|index| {
        matches!(value.as_bytes().get(index), Some(b'.' | b')'))
            && value[index + 1..]
                .chars()
                .next()
                .is_some_and(char::is_whitespace)
    })
}

fn has_timestamp_prefix(value: &str) -> bool {
    let token = value
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .trim_matches(['[', ']']);
    let Some((hours, minutes)) = token.split_once(':') else {
        return false;
    };

    hours.len() <= 2
        && !hours.is_empty()
        && minutes.len() == 2
        && hours.chars().all(|character| character.is_ascii_digit())
        && minutes.chars().all(|character| character.is_ascii_digit())
}

fn has_sender_prefix(value: &str) -> bool {
    if let Some(end) = value.find("]: ") {
        return value.starts_with('[') && end > 1;
    }
    value
        .split_once(": ")
        .is_some_and(|(prefix, _)| !prefix.is_empty() && !prefix.contains(' '))
}

pub fn segment_document(
    document: &RawDocument,
    options: &SegmentationOptions,
) -> Vec<RecordCandidate> {
    let normalized = normalize_document(document);
    segment_normalized_blocks(document, &normalized, options)
}

pub fn segment_document_with_repeated_identifier_markers(
    document: &RawDocument,
    markers: &[String],
) -> Vec<RecordCandidate> {
    let normalized = normalize_document(document);
    segment_repeated_identifier_blocks(document, &normalized, markers)
}

pub fn segment_normalized_blocks(
    document: &RawDocument,
    normalized: &[NormalizedBlock],
    options: &SegmentationOptions,
) -> Vec<RecordCandidate> {
    if options.strategy == SegmentationStrategy::SplitRepeatedIdentifiers {
        let markers = default_repeated_identifier_markers();
        return segment_repeated_identifier_blocks(document, normalized, &markers);
    }

    let groups = match options.strategy {
        SegmentationStrategy::OneBlockPerRecord => {
            normalized.iter().map(|block| vec![block]).collect()
        }
        SegmentationStrategy::OneRowPerRecord => group_by_row(document, normalized),
        SegmentationStrategy::JoinIndentedContinuations => group_indented_continuations(normalized),
        SegmentationStrategy::SplitRepeatedIdentifiers => {
            unreachable!("repeated identifier segmentation is handled before grouping")
        }
    };

    groups
        .into_iter()
        .enumerate()
        .map(|(index, group)| {
            let joined = group
                .iter()
                .map(|block| block.normalized_text.as_str())
                .collect::<Vec<_>>()
                .join(&options.join_separator);
            let source_block_ids = group
                .iter()
                .map(|block| block.source_block_id.clone())
                .collect();
            let heading_warning = if options.strategy == SegmentationStrategy::JoinIndentedContinuations
            {
                heading_following_warning(document, normalized, &group)
            } else {
                None
            };
            let (confidence, reason) = match options.strategy {
                SegmentationStrategy::OneBlockPerRecord => (
                    1.0,
                    Reason::new("one_block_boundary", "one source block is one record"),
                ),
                SegmentationStrategy::OneRowPerRecord => (
                    0.98,
                    Reason::new("one_row_boundary", "source cells share one row"),
                ),
                SegmentationStrategy::JoinIndentedContinuations if heading_warning.is_some() => (
                    0.35,
                    Reason::new(
                        "ambiguous_heading_continuation",
                        "indented text after a heading was kept separate because its record boundary is ambiguous",
                    ),
                ),
                SegmentationStrategy::JoinIndentedContinuations
                    if group.len() == 1 && is_heading_block(group[0]) => (
                    0.98,
                    Reason::new(
                        "heading_boundary",
                        "a heading-marked source block starts a visible section boundary",
                    ),
                ),
                SegmentationStrategy::JoinIndentedContinuations if group.len() > 1 => (
                    0.85,
                    Reason::new(
                        "indented_continuation",
                        "indented source blocks were joined to the preceding block",
                    ),
                ),
                SegmentationStrategy::JoinIndentedContinuations => (
                    1.0,
                    Reason::new("record_start", "no continuation evidence joined this block"),
                ),
                SegmentationStrategy::SplitRepeatedIdentifiers => unreachable!(
                    "repeated identifier segmentation is handled before candidate construction"
                ),
            };

            RecordCandidate {
                id: format!("record-{}", index + 1),
                source_block_ids,
                text: joined,
                confidence,
                reasons: vec![reason],
                warnings: heading_warning.into_iter().collect(),
            }
        })
        .collect()
}

const DEFAULT_REPEATED_IDENTIFIER_MARKERS: &[&str] = &["id:", "record:", "item:"];

#[derive(Debug, Clone, PartialEq, Eq)]
enum RepeatedIdentifierOutcome {
    Split(Vec<String>),
    NoEvidence,
    Ambiguous,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct IdentifierOccurrence {
    marker: String,
    start: usize,
}

fn default_repeated_identifier_markers() -> Vec<String> {
    DEFAULT_REPEATED_IDENTIFIER_MARKERS
        .iter()
        .map(|marker| (*marker).to_owned())
        .collect()
}

fn segment_repeated_identifier_blocks(
    document: &RawDocument,
    normalized: &[NormalizedBlock],
    markers: &[String],
) -> Vec<RecordCandidate> {
    let mut candidates = Vec::new();

    for block in normalized {
        match split_on_repeated_identifier(&block.normalized_text, markers) {
            RepeatedIdentifierOutcome::Split(parts) => {
                for part in parts {
                    candidates.push(RecordCandidate {
                        id: format!("record-{}", candidates.len() + 1),
                        source_block_ids: vec![block.source_block_id.clone()],
                        text: part,
                        confidence: 0.82,
                        reasons: vec![Reason::new(
                            "repeated_identifier_boundary",
                            "a repeated strong identifier marker established a record boundary",
                        )],
                        warnings: Vec::new(),
                    });
                }
            }
            RepeatedIdentifierOutcome::NoEvidence => {
                candidates.push(RecordCandidate {
                    id: format!("record-{}", candidates.len() + 1),
                    source_block_ids: vec![block.source_block_id.clone()],
                    text: block.normalized_text.clone(),
                    confidence: 0.9,
                    reasons: vec![Reason::new(
                        "no_repeated_identifier_boundary",
                        "no repeated strong identifier marker established a record boundary",
                    )],
                    warnings: Vec::new(),
                });
            }
            RepeatedIdentifierOutcome::Ambiguous => {
                candidates.push(RecordCandidate {
                    id: format!("record-{}", candidates.len() + 1),
                    source_block_ids: vec![block.source_block_id.clone()],
                    text: block.normalized_text.clone(),
                    confidence: 0.35,
                    reasons: vec![Reason::new(
                        "ambiguous_repeated_identifier_boundary",
                        "repeated identifier evidence did not establish a safe record boundary",
                    )],
                    warnings: vec![ParserWarning {
                        code: "ambiguous_repeated_identifier_boundary".to_owned(),
                        message: "the block was kept intact because repeated identifier evidence was ambiguous".to_owned(),
                        location: source_location(document, &block.source_block_id).cloned(),
                    }],
                });
            }
        }
    }

    candidates
}

fn split_on_repeated_identifier(text: &str, markers: &[String]) -> RepeatedIdentifierOutcome {
    let occurrences = identifier_occurrences(text, markers);
    if occurrences.is_empty() {
        return RepeatedIdentifierOutcome::NoEvidence;
    }

    let mut marker_positions: Vec<(String, Vec<usize>)> = Vec::new();
    for occurrence in occurrences {
        if let Some((_, positions)) = marker_positions
            .iter_mut()
            .find(|(marker, _)| marker == &occurrence.marker)
        {
            positions.push(occurrence.start);
        } else {
            marker_positions.push((occurrence.marker, vec![occurrence.start]));
        }
    }

    for (_, positions) in &mut marker_positions {
        positions.sort_unstable();
        positions.dedup();
    }

    let repeated_markers: Vec<_> = marker_positions
        .into_iter()
        .filter(|(_, positions)| positions.len() > 1)
        .collect();
    if repeated_markers.is_empty() {
        return RepeatedIdentifierOutcome::NoEvidence;
    }
    if repeated_markers.len() > 1 {
        return RepeatedIdentifierOutcome::Ambiguous;
    }

    let (marker, positions) = repeated_markers
        .into_iter()
        .next()
        .expect("repeated marker exists");
    if positions[0] != 0 {
        return RepeatedIdentifierOutcome::Ambiguous;
    }

    let parts = positions
        .iter()
        .enumerate()
        .map(|(index, start)| {
            let end = positions.get(index + 1).copied().unwrap_or(text.len());
            text[*start..end].trim().to_owned()
        })
        .collect::<Vec<_>>();
    let has_values = positions.iter().enumerate().all(|(index, start)| {
        let value_start = start + marker.len();
        let value_end = positions.get(index + 1).copied().unwrap_or(text.len());
        !text[value_start..value_end].trim().is_empty()
    });

    if !has_values || parts.iter().any(String::is_empty) {
        RepeatedIdentifierOutcome::Ambiguous
    } else {
        RepeatedIdentifierOutcome::Split(parts)
    }
}

fn identifier_occurrences(text: &str, markers: &[String]) -> Vec<IdentifierOccurrence> {
    let lowered_text = text.to_ascii_lowercase();
    let mut occurrences = Vec::new();

    for marker in markers {
        let marker = marker.trim().to_ascii_lowercase();
        if marker.is_empty() || (!marker.ends_with(':') && !marker.ends_with('=')) {
            continue;
        }

        let mut search_start = 0;
        while search_start < lowered_text.len() {
            let Some(relative_start) = lowered_text[search_start..].find(&marker) else {
                break;
            };
            let start = search_start + relative_start;
            let end = start + marker.len();
            if is_strong_identifier_occurrence(text, start, end) {
                occurrences.push(IdentifierOccurrence {
                    marker: marker.clone(),
                    start,
                });
            }
            search_start = end.max(start + 1);
        }
    }

    occurrences.sort_by(|left, right| {
        left.start
            .cmp(&right.start)
            .then_with(|| left.marker.len().cmp(&right.marker.len()))
            .then_with(|| left.marker.cmp(&right.marker))
    });
    occurrences
}

fn is_strong_identifier_occurrence(text: &str, start: usize, end: usize) -> bool {
    let before_is_boundary = text[..start].chars().next_back().is_none_or(|character| {
        character.is_whitespace() || matches!(character, '|' | ';' | ',' | '(' | '[')
    });
    let after_is_value = text[end..]
        .chars()
        .next()
        .is_some_and(|character| character.is_whitespace() || character == '=');

    before_is_boundary && after_is_value
}

fn group_by_row<'a>(
    document: &RawDocument,
    normalized: &'a [NormalizedBlock],
) -> Vec<Vec<&'a NormalizedBlock>> {
    let mut groups: Vec<Vec<&NormalizedBlock>> = Vec::new();

    for block in normalized {
        let joins_previous = groups
            .last()
            .and_then(|group| group.last())
            .is_some_and(|previous| same_source_row(document, previous, block));
        if joins_previous {
            groups.last_mut().expect("group exists").push(block);
        } else {
            groups.push(vec![block]);
        }
    }

    groups
}

fn group_indented_continuations(normalized: &[NormalizedBlock]) -> Vec<Vec<&NormalizedBlock>> {
    let mut groups: Vec<Vec<&NormalizedBlock>> = Vec::new();

    for block in normalized {
        let previous = groups.last().and_then(|group| group.last()).copied();
        let joins_previous = !block.normalized_text.is_empty()
            && has_leading_whitespace(&block.original)
            && previous.is_some()
            && !is_heading_block(block)
            && !previous.is_some_and(is_heading_block);
        if joins_previous {
            groups.last_mut().expect("group exists").push(block);
        } else {
            groups.push(vec![block]);
        }
    }

    groups
}

fn is_heading_block(block: &NormalizedBlock) -> bool {
    block
        .transformations
        .contains(&Transformation::HeadingDetected)
}

fn heading_following_warning(
    document: &RawDocument,
    normalized: &[NormalizedBlock],
    group: &[&NormalizedBlock],
) -> Option<ParserWarning> {
    let [block] = group else {
        return None;
    };
    if !has_leading_whitespace(&block.original) {
        return None;
    }

    let block_index = normalized
        .iter()
        .position(|candidate| candidate.source_block_id == block.source_block_id)?;
    let previous = block_index
        .checked_sub(1)
        .and_then(|index| normalized.get(index))?;
    if !is_heading_block(previous) {
        return None;
    }

    Some(ParserWarning {
        code: "ambiguous_heading_continuation".to_owned(),
        message: "indented text after a heading was kept separate because its record boundary is ambiguous".to_owned(),
        location: source_location(document, &block.source_block_id).cloned(),
    })
}

fn has_leading_whitespace(value: &RawValue) -> bool {
    value
        .to_text()
        .chars()
        .next()
        .is_some_and(char::is_whitespace)
}

fn same_source_row(
    document: &RawDocument,
    first: &NormalizedBlock,
    second: &NormalizedBlock,
) -> bool {
    let Some(first_location) = source_location(document, &first.source_block_id) else {
        return false;
    };
    let Some(second_location) = source_location(document, &second.source_block_id) else {
        return false;
    };

    first_location.row.is_some()
        && first_location.row == second_location.row
        && first_location.sheet == second_location.sheet
}

fn source_location<'a>(
    document: &'a RawDocument,
    source_block_id: &str,
) -> Option<&'a SourceLocation> {
    document
        .blocks
        .iter()
        .find(|block| block.id == source_block_id)
        .map(|block| &block.location)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IoErrorKind {
    NotFound,
    PermissionDenied,
    InvalidInput,
    Other,
}

impl From<io::ErrorKind> for IoErrorKind {
    fn from(kind: io::ErrorKind) -> Self {
        match kind {
            io::ErrorKind::NotFound => Self::NotFound,
            io::ErrorKind::PermissionDenied => Self::PermissionDenied,
            io::ErrorKind::InvalidInput => Self::InvalidInput,
            _ => Self::Other,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "code")]
pub enum ParserError {
    #[serde(rename = "io_error")]
    Io { path: String, kind: IoErrorKind },
    #[serde(rename = "invalid_utf8")]
    InvalidUtf8 { path: String, valid_up_to: usize },
    #[serde(rename = "unsupported_input")]
    UnsupportedInput { source_type: String },
    #[serde(rename = "input_too_large")]
    InputTooLarge {
        source: String,
        limit: usize,
        actual: usize,
    },
    #[serde(rename = "line_too_long")]
    LineTooLong {
        source: String,
        line: usize,
        limit: usize,
        actual: usize,
    },
    #[serde(rename = "invalid_csv")]
    InvalidCsv {
        path: String,
        record: Option<usize>,
        message: String,
    },
    #[serde(rename = "invalid_xlsx")]
    InvalidXlsx { path: String, message: String },
}

impl ParserError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Io { .. } => "io_error",
            Self::InvalidUtf8 { .. } => "invalid_utf8",
            Self::UnsupportedInput { .. } => "unsupported_input",
            Self::InputTooLarge { .. } => "input_too_large",
            Self::LineTooLong { .. } => "line_too_long",
            Self::InvalidCsv { .. } => "invalid_csv",
            Self::InvalidXlsx { .. } => "invalid_xlsx",
        }
    }
}

impl fmt::Display for ParserError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, kind } => write!(formatter, "could not read {path}: {kind:?}"),
            Self::InvalidUtf8 { path, valid_up_to } => write!(
                formatter,
                "{path} is not valid UTF-8 at byte offset {valid_up_to}"
            ),
            Self::UnsupportedInput { source_type } => {
                write!(formatter, "unsupported input type: {source_type}")
            }
            Self::InputTooLarge {
                source,
                limit,
                actual,
            } => write!(
                formatter,
                "{source} exceeds the {limit}-byte limit ({actual} bytes)"
            ),
            Self::LineTooLong {
                source,
                line,
                limit,
                actual,
            } => write!(
                formatter,
                "{source} line {line} exceeds the {limit}-byte limit ({actual} bytes)"
            ),
            Self::InvalidCsv {
                path,
                record,
                message,
            } => match record {
                Some(record) => write!(
                    formatter,
                    "invalid CSV in {path} at record {record}: {message}"
                ),
                None => write!(formatter, "invalid CSV in {path}: {message}"),
            },
            Self::InvalidXlsx { path, message } => {
                write!(formatter, "invalid XLSX in {path}: {message}")
            }
        }
    }
}

impl std::error::Error for ParserError {}

pub fn core_ready() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_document(blocks: Vec<RawBlock>) -> RawDocument {
        RawDocument::new(
            "test-document",
            SourceMetadata {
                source_type: SourceType::Text,
                file_name: None,
                mime_type: Some("text/plain".to_owned()),
                size_bytes: None,
                delimiter: None,
            },
            blocks,
        )
    }

    #[test]
    fn raw_document_round_trips_as_json() {
        let document = RawDocument::new(
            "document-1",
            SourceMetadata {
                source_type: SourceType::Txt,
                file_name: Some("sample.txt".to_owned()),
                mime_type: Some("text/plain".to_owned()),
                size_bytes: Some(5),
                delimiter: None,
            },
            vec![RawBlock {
                id: "block-1".to_owned(),
                value: RawValue::text("Ada  Lovelace"),
                location: SourceLocation {
                    line: Some(1),
                    byte_start: Some(0),
                    byte_end: Some(13),
                    ..SourceLocation::default()
                },
            }],
        );

        let json = serde_json::to_string(&document).expect("document should serialize");
        let decoded: RawDocument =
            serde_json::from_str(&json).expect("document should deserialize");

        assert_eq!(decoded, document);
    }

    #[test]
    fn email_detection_preserves_value_and_byte_span() {
        let text = "Contact: Ada ada@example.test.";
        let candidates = detect_email_candidates(text);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].candidate_type, CandidateType::Email);
        assert_eq!(candidates[0].raw_value, "ada@example.test");
        assert_eq!(
            &text[candidates[0].source_span.byte_start..candidates[0].source_span.byte_end],
            "ada@example.test"
        );
        assert_eq!(
            candidates[0].normalized_value,
            Some(serde_json::Value::String("ada@example.test".to_owned()))
        );
    }

    #[test]
    fn email_detection_ignores_near_misses() {
        assert!(detect_email_candidates("missing-at.example invalid@localhost").is_empty());
    }

    #[test]
    fn integer_detection_returns_normalized_values_and_spans() {
        let text = "count: -42, next 7.";
        let candidates = detect_integer_candidates(text);

        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].raw_value, "-42");
        assert_eq!(candidates[0].normalized_value, Some(serde_json::json!(-42)));
        assert_eq!(
            &text[candidates[0].source_span.byte_start..candidates[0].source_span.byte_end],
            "-42"
        );
        assert_eq!(candidates[1].raw_value, "7");
    }

    #[test]
    fn integer_detection_does_not_extract_digits_from_mixed_tokens() {
        assert!(detect_integer_candidates("phone 555-0123 room12").is_empty());
    }

    #[test]
    fn decimal_detection_excludes_integers_and_normalizes_values() {
        let text = "whole 7 decimal -12.50, invalid 1.2.3";
        let candidates = detect_decimal_candidates(text);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].candidate_type, CandidateType::Decimal);
        assert_eq!(candidates[0].raw_value, "-12.50");
        assert_eq!(
            candidates[0].normalized_value,
            Some(serde_json::json!(-12.5))
        );
    }

    #[test]
    fn phone_detection_normalizes_separators_and_preserves_span() {
        let text = "call +1-555-012-3456.";
        let candidates = detect_phone_candidates(text);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].candidate_type, CandidateType::PhoneNumber);
        assert_eq!(candidates[0].raw_value, "+1-555-012-3456");
        assert_eq!(
            candidates[0].normalized_value,
            Some(serde_json::json!("15550123456"))
        );
        assert_eq!(
            &text[candidates[0].source_span.byte_start..candidates[0].source_span.byte_end],
            "+1-555-012-3456"
        );
    }

    #[test]
    fn phone_detection_ignores_short_and_mixed_tokens() {
        assert!(detect_phone_candidates("room 12345 code A5550123").is_empty());
    }

    #[test]
    fn currency_detection_normalizes_symbol_amounts_and_preserves_span() {
        let text = "Total: $12.50, other EUR 9.00";
        let candidates = detect_currency_candidates(text);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].candidate_type, CandidateType::Currency);
        assert_eq!(candidates[0].raw_value, "$12.50");
        assert_eq!(
            candidates[0].normalized_value,
            Some(serde_json::json!(12.5))
        );
        assert_eq!(
            &text[candidates[0].source_span.byte_start..candidates[0].source_span.byte_end],
            "$12.50"
        );
    }

    #[test]
    fn currency_detection_ignores_unmarked_amounts() {
        assert!(detect_currency_candidates("amount 12.50 dollars").is_empty());
    }

    #[test]
    fn enum_detection_normalizes_aliases_to_canonical_values() {
        let definitions = vec![(
            "active".to_owned(),
            vec!["enabled".to_owned(), "on".to_owned()],
        )];
        let text = "Status: ENABLED.";
        let candidates = detect_enum_candidates(text, &definitions);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].candidate_type, CandidateType::Enum);
        assert_eq!(candidates[0].raw_value, "ENABLED");
        assert_eq!(
            candidates[0].normalized_value,
            Some(serde_json::json!("active"))
        );
        assert_eq!(
            &text[candidates[0].source_span.byte_start..candidates[0].source_span.byte_end],
            "ENABLED"
        );
    }

    #[test]
    fn enum_detection_ignores_values_without_definitions() {
        let definitions = vec![("active".to_owned(), vec!["enabled".to_owned()])];
        assert!(detect_enum_candidates("pending unknown", &definitions).is_empty());
    }

    #[test]
    fn assignment_selects_highest_confidence_compatible_candidate() {
        let mut candidates = detect_email_candidates("first a@example.test second b@example.test");
        candidates[0].confidence = 0.8;
        let result = assign_candidates(
            "first a@example.test second b@example.test",
            &candidates,
            &[AssignmentField {
                name: "email".to_owned(),
                aliases: Vec::new(),
                candidate_type: CandidateType::Email,
                required: true,
                multiple: false,
                unique: true,
                constraints: Vec::new(),
                expected_column: None,
            }],
        );

        assert_eq!(result.fields.len(), 1);
        assert_eq!(result.fields[0].candidates[0].raw_value, "b@example.test");
        assert_eq!(result.unassigned_candidates.len(), 1);
        assert_eq!(result.warnings[0].code, "multiple_candidates_ambiguous");
    }

    #[test]
    fn assignment_prefers_nearby_field_label_over_confidence_alone() {
        let text = "backup a@example.test Email: b@example.test";
        let mut candidates = detect_email_candidates(text);
        candidates[1].confidence = 0.8;
        let result = assign_candidates(
            text,
            &candidates,
            &[AssignmentField {
                name: "email".to_owned(),
                aliases: vec!["contact".to_owned()],
                candidate_type: CandidateType::Email,
                required: true,
                multiple: false,
                unique: true,
                constraints: Vec::new(),
                expected_column: None,
            }],
        );

        assert_eq!(result.fields[0].candidates[0].raw_value, "b@example.test");
    }

    #[test]
    fn assignment_prefers_expected_column_context_over_confidence_alone() {
        let text = "first a@example.test second b@example.test";
        let mut candidates = detect_email_candidates(text);
        candidates[0].source_column = Some(1);
        candidates[1].source_column = Some(2);
        candidates[1].confidence = 0.8;
        let result = assign_candidates(
            text,
            &candidates,
            &[AssignmentField {
                name: "email".to_owned(),
                aliases: Vec::new(),
                candidate_type: CandidateType::Email,
                required: true,
                multiple: false,
                unique: true,
                constraints: Vec::new(),
                expected_column: Some(2),
            }],
        );

        assert_eq!(result.fields[0].candidates[0].raw_value, "b@example.test");
    }

    #[test]
    fn assignment_reports_missing_required_and_unassigned_candidates() {
        let candidates = detect_integer_candidates("count 4");
        let result = assign_candidates(
            "count 4",
            &candidates,
            &[AssignmentField {
                name: "email".to_owned(),
                aliases: Vec::new(),
                candidate_type: CandidateType::Email,
                required: true,
                multiple: false,
                unique: true,
                constraints: Vec::new(),
                expected_column: None,
            }],
        );

        assert!(result.fields.is_empty());
        assert_eq!(result.unassigned_candidates.len(), 1);
        assert_eq!(result.warnings[0].code, "required_field_missing");
    }

    #[test]
    fn assignment_keeps_all_compatible_candidates_for_multiple_fields() {
        let candidates = detect_integer_candidates("first 4 second 7");
        let result = assign_candidates(
            "first 4 second 7",
            &candidates,
            &[AssignmentField {
                name: "counts".to_owned(),
                aliases: Vec::new(),
                candidate_type: CandidateType::Integer,
                required: false,
                multiple: true,
                unique: false,
                constraints: Vec::new(),
                expected_column: None,
            }],
        );

        assert_eq!(result.fields[0].candidates.len(), 2);
        assert!(result.warnings.is_empty());
        assert!(result.unassigned_candidates.is_empty());
    }

    #[test]
    fn assignment_filters_candidates_that_violate_integer_constraints() {
        let text = "small 3 valid 12";
        let candidates = detect_integer_candidates(text);
        let result = assign_candidates(
            text,
            &candidates,
            &[AssignmentField {
                name: "quantity".to_owned(),
                aliases: Vec::new(),
                candidate_type: CandidateType::Integer,
                required: true,
                multiple: false,
                unique: true,
                constraints: vec![AssignmentConstraint::MinimumInteger(10)],
                expected_column: None,
            }],
        );

        assert_eq!(result.fields[0].candidates[0].raw_value, "12");
        assert_eq!(result.unassigned_candidates[0].raw_value, "3");
    }

    #[test]
    fn assignment_reports_missing_required_when_all_candidates_violate_constraints() {
        let text = "quantity 3";
        let candidates = detect_integer_candidates(text);
        let result = assign_candidates(
            text,
            &candidates,
            &[AssignmentField {
                name: "quantity".to_owned(),
                aliases: Vec::new(),
                candidate_type: CandidateType::Integer,
                required: true,
                multiple: false,
                unique: true,
                constraints: vec![AssignmentConstraint::MinimumInteger(10)],
                expected_column: None,
            }],
        );

        assert!(result.fields.is_empty());
        assert_eq!(result.unassigned_candidates.len(), 1);
        assert_eq!(result.warnings[0].code, "required_field_missing");
    }

    #[test]
    fn assignment_models_round_trip_through_json() {
        let field = AssignmentField {
            name: "quantity".to_owned(),
            aliases: vec!["count".to_owned()],
            candidate_type: CandidateType::Integer,
            required: true,
            multiple: false,
            unique: true,
            constraints: vec![AssignmentConstraint::MinimumInteger(1)],
            expected_column: None,
        };
        let result = assign_candidates(
            "quantity: 4",
            &detect_integer_candidates("quantity: 4"),
            &[field],
        );
        let json = serde_json::to_string(&result).expect("assignment should serialize");
        let decoded: AssignmentResult =
            serde_json::from_str(&json).expect("assignment should deserialize");

        assert_eq!(decoded, result);
    }

    #[test]
    fn text_pipeline_detects_and_assigns_caller_defined_fields() {
        let fields = vec![
            AssignmentField {
                name: "email".to_owned(),
                aliases: Vec::new(),
                candidate_type: CandidateType::Email,
                required: true,
                multiple: false,
                unique: true,
                constraints: Vec::new(),
                expected_column: None,
            },
            AssignmentField {
                name: "status".to_owned(),
                aliases: Vec::new(),
                candidate_type: CandidateType::Enum,
                required: true,
                multiple: false,
                unique: true,
                constraints: Vec::new(),
                expected_column: None,
            },
        ];
        let result = parse_text_with_assignment(
            "Email: ada@example.test Status: ENABLED",
            &fields,
            &[("active".to_owned(), vec!["enabled".to_owned()])],
        );

        assert_eq!(result.candidates.len(), 2);
        assert_eq!(result.assignment.fields.len(), 2);
        assert!(result.assignment.unassigned_candidates.is_empty());
        assert!(result.assignment.warnings.is_empty());
    }

    #[test]
    fn boolean_detection_normalizes_common_aliases() {
        let candidates = detect_boolean_candidates("Enabled: YES disabled: off maybe");

        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].raw_value, "YES");
        assert_eq!(
            candidates[0].normalized_value,
            Some(serde_json::json!(true))
        );
        assert_eq!(candidates[1].raw_value, "off");
        assert_eq!(
            candidates[1].normalized_value,
            Some(serde_json::json!(false))
        );
    }

    #[test]
    fn boolean_detection_ignores_embedded_aliases() {
        assert!(detect_boolean_candidates("yesterday onboard truthful").is_empty());
    }

    #[test]
    fn date_detection_normalizes_supported_formats() {
        let text = "started 2026-08-23, renewed 2027/01/05";
        let candidates = detect_date_candidates(text);

        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].raw_value, "2026-08-23");
        assert_eq!(
            candidates[1].normalized_value,
            Some(serde_json::json!("2027-01-05"))
        );
    }

    #[test]
    fn date_detection_rejects_invalid_calendar_values() {
        assert!(detect_date_candidates("2026-02-29 2026-13-01 26-01-01").is_empty());
    }

    #[test]
    fn parser_errors_have_stable_codes() {
        let error = ParserError::InvalidUtf8 {
            path: "input.txt".to_owned(),
            valid_up_to: 4,
        };

        assert_eq!(error.code(), "invalid_utf8");
        assert_eq!(
            error.to_string(),
            "input.txt is not valid UTF-8 at byte offset 4"
        );
    }

    #[test]
    fn input_limits_have_stable_codes() {
        let error = ParserError::LineTooLong {
            source: "<stdin>".to_owned(),
            line: 3,
            limit: 10,
            actual: 11,
        };

        assert_eq!(error.code(), "line_too_long");
        assert_eq!(
            error.to_string(),
            "<stdin> line 3 exceeds the 10-byte limit (11 bytes)"
        );
    }

    #[test]
    fn normalization_preserves_raw_value_and_records_transforms() {
        let block = RawBlock {
            id: "block-1".to_owned(),
            value: RawValue::text("  Ada  —  “Lovelace”\r\n"),
            location: SourceLocation::default(),
        };

        let normalized = normalize_block(&block);

        assert_eq!(normalized.source_block_id, "block-1");
        assert_eq!(normalized.original, block.value);
        assert_eq!(normalized.normalized_text, "Ada - \"Lovelace\"");
        assert_eq!(
            normalized.transformations,
            vec![
                Transformation::LineEndingsNormalized,
                Transformation::DashesNormalized,
                Transformation::QuotesNormalized,
                Transformation::WhitespaceTrimmed,
                Transformation::WhitespaceCollapsed,
            ]
        );

        let json = serde_json::to_string(&normalized).expect("normalized block should serialize");
        let decoded: NormalizedBlock =
            serde_json::from_str(&json).expect("normalized block should deserialize");
        assert_eq!(decoded, normalized);
    }

    #[test]
    fn normalization_marks_noise_without_removing_it() {
        let cases = [
            ("- item", Transformation::ListMarkerDetected),
            ("12. item", Transformation::ListMarkerDetected),
            ("[12:30] Alice", Transformation::TimestampPrefixDetected),
            ("[Alice]: value", Transformation::SenderPrefixDetected),
            ("# Heading", Transformation::HeadingDetected),
        ];

        for (value, expected) in cases {
            let block = RawBlock {
                id: value.to_owned(),
                value: RawValue::text(value),
                location: SourceLocation::default(),
            };
            let normalized = normalize_block(&block);

            assert_eq!(normalized.normalized_text, value);
            assert!(normalized.transformations.contains(&expected));
        }
    }

    #[test]
    fn normalization_options_can_disable_derived_changes() {
        let block = RawBlock {
            id: "block-1".to_owned(),
            value: RawValue::text("  Ada  —  Lovelace  "),
            location: SourceLocation::default(),
        };
        let options = NormalizationOptions {
            normalize_line_endings: false,
            trim_whitespace: false,
            collapse_whitespace: false,
            normalize_punctuation: false,
            mark_noise: false,
        };

        let normalized = normalize_block_with_options(&block, &options);

        assert_eq!(normalized.normalized_text, "  Ada  —  Lovelace  ");
        assert!(normalized.transformations.is_empty());
        assert_eq!(normalized.original, block.value);
    }

    #[test]
    fn normalization_converts_typed_values_without_replacing_originals() {
        let block = RawBlock {
            id: "number".to_owned(),
            value: RawValue::Integer(42),
            location: SourceLocation::default(),
        };

        let normalized = normalize_block(&block);

        assert_eq!(normalized.normalized_text, "42");
        assert_eq!(normalized.original, RawValue::Integer(42));
    }

    #[test]
    fn one_block_strategy_produces_traceable_candidates() {
        let document = test_document(vec![
            RawBlock {
                id: "block-1".to_owned(),
                value: RawValue::text("Ada"),
                location: SourceLocation {
                    line: Some(1),
                    ..SourceLocation::default()
                },
            },
            RawBlock {
                id: "block-2".to_owned(),
                value: RawValue::text("Grace"),
                location: SourceLocation {
                    line: Some(2),
                    ..SourceLocation::default()
                },
            },
        ]);

        let candidates = segment_document(&document, &SegmentationOptions::default());

        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].id, "record-1");
        assert_eq!(candidates[0].source_block_ids, vec!["block-1"]);
        assert_eq!(candidates[0].text, "Ada");
        assert_eq!(candidates[0].confidence, 1.0);
        assert_eq!(candidates[0].reasons[0].code, "one_block_boundary");
    }

    #[test]
    fn one_row_strategy_groups_cells_without_losing_provenance() {
        let document = test_document(vec![
            RawBlock {
                id: "row-1-column-1".to_owned(),
                value: RawValue::text("Ada"),
                location: SourceLocation {
                    row: Some(1),
                    column: Some(1),
                    ..SourceLocation::default()
                },
            },
            RawBlock {
                id: "row-1-column-2".to_owned(),
                value: RawValue::text("ada@example.test"),
                location: SourceLocation {
                    row: Some(1),
                    column: Some(2),
                    ..SourceLocation::default()
                },
            },
            RawBlock {
                id: "row-2-column-1".to_owned(),
                value: RawValue::text("Grace"),
                location: SourceLocation {
                    row: Some(2),
                    column: Some(1),
                    ..SourceLocation::default()
                },
            },
        ]);
        let options = SegmentationOptions {
            strategy: SegmentationStrategy::OneRowPerRecord,
            join_separator: " | ".to_owned(),
        };

        let candidates = segment_document(&document, &options);

        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].text, "Ada | ada@example.test");
        assert_eq!(
            candidates[0].source_block_ids,
            vec!["row-1-column-1", "row-1-column-2"]
        );
        assert_eq!(candidates[0].confidence, 0.98);
        assert_eq!(candidates[1].text, "Grace");
    }

    #[test]
    fn indented_continuations_join_with_lower_confidence() {
        let document = test_document(vec![
            RawBlock {
                id: "line-1".to_owned(),
                value: RawValue::text("Name: Ada"),
                location: SourceLocation::default(),
            },
            RawBlock {
                id: "line-2".to_owned(),
                value: RawValue::text("  email: ada@example.test"),
                location: SourceLocation::default(),
            },
            RawBlock {
                id: "line-3".to_owned(),
                value: RawValue::text("Name: Grace"),
                location: SourceLocation::default(),
            },
        ]);
        let options = SegmentationOptions {
            strategy: SegmentationStrategy::JoinIndentedContinuations,
            join_separator: "\n".to_owned(),
        };

        let candidates = segment_document(&document, &options);

        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].text, "Name: Ada\nemail: ada@example.test");
        assert_eq!(candidates[0].confidence, 0.85);
        assert_eq!(candidates[0].reasons[0].code, "indented_continuation");
        assert_eq!(candidates[1].text, "Name: Grace");
    }

    #[test]
    fn heading_boundaries_keep_sections_observable_and_warn_on_indented_followers() {
        let document = test_document(vec![
            RawBlock {
                id: "line-1".to_owned(),
                value: RawValue::text("Name: Ada"),
                location: SourceLocation {
                    line: Some(1),
                    ..SourceLocation::default()
                },
            },
            RawBlock {
                id: "line-2".to_owned(),
                value: RawValue::text("  email: ada@example.test"),
                location: SourceLocation {
                    line: Some(2),
                    ..SourceLocation::default()
                },
            },
            RawBlock {
                id: "line-3".to_owned(),
                value: RawValue::text("# Section"),
                location: SourceLocation {
                    line: Some(3),
                    ..SourceLocation::default()
                },
            },
            RawBlock {
                id: "line-4".to_owned(),
                value: RawValue::text("  section text"),
                location: SourceLocation {
                    line: Some(4),
                    ..SourceLocation::default()
                },
            },
        ]);
        let options = SegmentationOptions {
            strategy: SegmentationStrategy::JoinIndentedContinuations,
            join_separator: "\n".to_owned(),
        };

        let candidates = segment_document(&document, &options);

        assert_eq!(candidates.len(), 3);
        assert_eq!(candidates[0].text, "Name: Ada\nemail: ada@example.test");
        assert_eq!(candidates[1].text, "# Section");
        assert_eq!(candidates[1].reasons[0].code, "heading_boundary");
        assert_eq!(candidates[2].text, "section text");
        assert_eq!(candidates[2].confidence, 0.35);
        assert_eq!(
            candidates[2].warnings[0].code,
            "ambiguous_heading_continuation"
        );
        assert_eq!(
            candidates[2].warnings[0]
                .location
                .as_ref()
                .and_then(|location| location.line),
            Some(4)
        );
    }

    #[test]
    fn indented_heading_does_not_join_the_previous_record() {
        let document = test_document(vec![
            RawBlock {
                id: "line-1".to_owned(),
                value: RawValue::text("Name: Ada"),
                location: SourceLocation::default(),
            },
            RawBlock {
                id: "line-2".to_owned(),
                value: RawValue::text("  # Section"),
                location: SourceLocation::default(),
            },
        ]);
        let options = SegmentationOptions {
            strategy: SegmentationStrategy::JoinIndentedContinuations,
            join_separator: "\n".to_owned(),
        };

        let candidates = segment_document(&document, &options);

        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[1].text, "# Section");
        assert_eq!(candidates[1].reasons[0].code, "heading_boundary");
    }

    #[test]
    fn repeated_identifier_strategy_splits_one_block_without_losing_source_reference() {
        let document = test_document(vec![RawBlock {
            id: "line-1".to_owned(),
            value: RawValue::text("ID: first ID: second"),
            location: SourceLocation {
                line: Some(1),
                ..SourceLocation::default()
            },
        }]);
        let options = SegmentationOptions {
            strategy: SegmentationStrategy::SplitRepeatedIdentifiers,
            join_separator: " | ".to_owned(),
        };

        let candidates = segment_document(&document, &options);

        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].text, "ID: first");
        assert_eq!(candidates[1].text, "ID: second");
        assert_eq!(candidates[0].source_block_ids, vec!["line-1"]);
        assert_eq!(candidates[1].source_block_ids, vec!["line-1"]);
        assert_eq!(candidates[0].confidence, 0.82);
        assert_eq!(
            candidates[0].reasons[0].code,
            "repeated_identifier_boundary"
        );
    }

    #[test]
    fn repeated_identifier_strategy_keeps_near_miss_intact() {
        let document = test_document(vec![RawBlock {
            id: "line-1".to_owned(),
            value: RawValue::text("ID: first identifier: second"),
            location: SourceLocation::default(),
        }]);
        let options = SegmentationOptions {
            strategy: SegmentationStrategy::SplitRepeatedIdentifiers,
            join_separator: "\n".to_owned(),
        };

        let candidates = segment_document(&document, &options);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].text, "ID: first identifier: second");
        assert_eq!(candidates[0].confidence, 0.9);
        assert!(candidates[0].warnings.is_empty());
    }

    #[test]
    fn repeated_identifier_strategy_reports_ambiguous_marker_sets() {
        let document = test_document(vec![RawBlock {
            id: "line-1".to_owned(),
            value: RawValue::text("ID: first ID: second Record: third Record: fourth"),
            location: SourceLocation {
                line: Some(7),
                ..SourceLocation::default()
            },
        }]);
        let options = SegmentationOptions {
            strategy: SegmentationStrategy::SplitRepeatedIdentifiers,
            join_separator: "\n".to_owned(),
        };

        let candidates = segment_document(&document, &options);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].confidence, 0.35);
        assert_eq!(
            candidates[0].reasons[0].code,
            "ambiguous_repeated_identifier_boundary"
        );
        assert_eq!(
            candidates[0].warnings[0]
                .location
                .as_ref()
                .and_then(|location| location.line),
            Some(7)
        );
    }

    #[test]
    fn repeated_identifier_markers_can_be_supplied_by_the_caller() {
        let document = test_document(vec![RawBlock {
            id: "line-1".to_owned(),
            value: RawValue::text("Ref: first Ref: second"),
            location: SourceLocation::default(),
        }]);
        let markers = vec!["Ref:".to_owned()];

        let candidates = segment_document_with_repeated_identifier_markers(&document, &markers);

        assert_eq!(
            candidates
                .iter()
                .map(|candidate| candidate.text.as_str())
                .collect::<Vec<_>>(),
            vec!["Ref: first", "Ref: second"]
        );
    }

    #[test]
    fn segmentation_serializes_candidates_and_keeps_blank_blocks() {
        let document = test_document(vec![RawBlock {
            id: "blank".to_owned(),
            value: RawValue::text(""),
            location: SourceLocation::default(),
        }]);

        let candidates = segment_document(&document, &SegmentationOptions::default());
        let json = serde_json::to_string(&candidates[0]).expect("candidate should serialize");
        let decoded: RecordCandidate =
            serde_json::from_str(&json).expect("candidate should deserialize");

        assert_eq!(decoded, candidates[0]);
        assert_eq!(decoded.source_block_ids, vec!["blank"]);
        assert_eq!(decoded.text, "");
    }

    #[test]
    fn io_error_kind_is_serializable() {
        let error = ParserError::Io {
            path: "missing.txt".to_owned(),
            kind: IoErrorKind::NotFound,
        };

        let json = serde_json::to_string(&error).expect("error should serialize");
        assert_eq!(
            json,
            r#"{"code":"io_error","path":"missing.txt","kind":"not_found"}"#
        );
    }

    #[test]
    fn empty_core_test() {
        assert!(core_ready());
    }

    fn table_block(sheet: Option<&str>, row: usize, column: usize, value: RawValue) -> RawBlock {
        RawBlock {
            id: format!("block-{row}-{column}"),
            value,
            location: SourceLocation {
                row: Some(row),
                column: Some(column),
                sheet: sheet.map(str::to_owned),
                ..SourceLocation::default()
            },
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn field_of(
        name: &str,
        aliases: &[&str],
        candidate_type: CandidateType,
        required: bool,
        multiple: bool,
    ) -> AssignmentField {
        AssignmentField {
            name: name.to_owned(),
            aliases: aliases.iter().map(|alias| alias.to_string()).collect(),
            candidate_type,
            required,
            multiple,
            unique: false,
            constraints: Vec::new(),
            expected_column: None,
        }
    }

    fn table_document(blocks: Vec<RawBlock>) -> RawDocument {
        RawDocument::new(
            "table-document",
            SourceMetadata {
                source_type: SourceType::Csv,
                file_name: Some("table.csv".to_owned()),
                mime_type: Some("text/csv".to_owned()),
                size_bytes: None,
                delimiter: Some(",".to_owned()),
            },
            blocks,
        )
    }

    #[test]
    fn table_headers_are_detected_and_drive_row_assignment() {
        let document = table_document(vec![
            table_block(None, 1, 1, RawValue::text("Email")),
            table_block(None, 1, 2, RawValue::text("Age")),
            table_block(None, 2, 1, RawValue::text("ada@example.test")),
            table_block(None, 2, 2, RawValue::Integer(30)),
        ]);
        let fields = [
            field_of("email", &[], CandidateType::Email, true, false),
            field_of("age", &[], CandidateType::Integer, false, false),
        ];

        let result = parse_document_rows_with_assignment(&document, &fields, &[]);

        assert_eq!(result.sheets.len(), 1);
        let sheet = &result.sheets[0];
        let HeaderExtraction::Detected { headers } = &sheet.header else {
            panic!("expected a detected header");
        };
        assert_eq!(headers.source_row, 1);
        assert_eq!(
            headers.labels,
            vec![(1, "Email".to_owned()), (2, "Age".to_owned())]
        );
        assert_eq!(sheet.records.len(), 1);
        let record = &sheet.records[0];
        assert_eq!(record.source_row, 2);
        assert_eq!(record.parse.assignment.fields.len(), 2);
        assert_eq!(
            record.parse.assignment.fields[0].candidates[0].raw_value,
            "ada@example.test"
        );
        assert_eq!(
            record.parse.assignment.fields[0].candidates[0].source_column,
            Some(1)
        );
        assert_eq!(
            record.parse.assignment.fields[1].candidates[0].normalized_value,
            Some(serde_json::json!(30))
        );
    }

    #[test]
    fn typed_first_rows_are_not_headers_and_stay_records() {
        let document = table_document(vec![
            table_block(None, 1, 1, RawValue::text("2024-01-15")),
            table_block(None, 1, 2, RawValue::text("Note")),
            table_block(None, 2, 1, RawValue::text("Contact")),
            table_block(None, 2, 2, RawValue::text("ada@example.test")),
        ]);
        let fields = [field_of("email", &[], CandidateType::Email, false, false)];

        let result = parse_document_rows_with_assignment(&document, &fields, &[]);

        let sheet = &result.sheets[0];
        let HeaderExtraction::NotDetected { code, .. } = &sheet.header else {
            panic!("expected header detection to be rejected");
        };
        assert_eq!(code, "header_not_detected_strong_values");
        assert_eq!(sheet.records.len(), 2);
        // Without a header the email still assigns by type compatibility.
        assert_eq!(
            sheet.records[1].parse.assignment.fields[0].candidates[0].raw_value,
            "ada@example.test"
        );
    }

    #[test]
    fn single_row_documents_do_not_invent_a_header() {
        let document = table_document(vec![
            table_block(None, 1, 1, RawValue::text("Email")),
            table_block(None, 1, 2, RawValue::text("Age")),
        ]);

        let result = parse_document_rows_with_assignment(&document, &[], &[]);

        let sheet = &result.sheets[0];
        let HeaderExtraction::NotDetected { code, message } = &sheet.header else {
            panic!("expected header detection to be rejected");
        };
        assert_eq!(code, "header_not_detected_single_row");
        assert!(!message.is_empty());
        assert_eq!(sheet.records.len(), 1);
    }

    #[test]
    fn header_labels_resolve_ambiguous_integer_columns() {
        let document = table_document(vec![
            table_block(None, 1, 1, RawValue::text("Quantity")),
            table_block(None, 1, 2, RawValue::text("Age")),
            table_block(None, 2, 1, RawValue::Integer(30)),
            table_block(None, 2, 2, RawValue::Integer(7)),
        ]);
        let fields = [field_of("age", &[], CandidateType::Integer, true, false)];

        let result = parse_document_rows_with_assignment(&document, &fields, &[]);

        let record = &result.sheets[0].records[0];
        let assignment = &record.parse.assignment;
        // Both integers are type-compatible; only the "Age" column matches.
        assert_eq!(assignment.warnings.len(), 0);
        assert_eq!(assignment.fields[0].candidates.len(), 1);
        let selected = &assignment.fields[0].candidates[0];
        assert_eq!(selected.source_column, Some(2));
        assert_eq!(selected.normalized_value, Some(serde_json::json!(7)));
        assert!(
            selected
                .reasons
                .iter()
                .any(|reason| reason.code == "header_label_match")
        );
        assert_eq!(assignment.unassigned_candidates.len(), 1);
    }

    #[test]
    fn multiple_fields_prefer_header_matching_columns() {
        let document = table_document(vec![
            table_block(None, 1, 1, RawValue::text("Phone")),
            table_block(None, 1, 2, RawValue::text("Fax")),
            table_block(None, 2, 1, RawValue::text("+15550101")),
            table_block(None, 2, 2, RawValue::text("+15550102")),
        ]);
        let fields = [field_of(
            "phone",
            &[],
            CandidateType::PhoneNumber,
            false,
            true,
        )];

        let result = parse_document_rows_with_assignment(&document, &fields, &[]);

        let assignment = &result.sheets[0].records[0].parse.assignment;
        assert_eq!(assignment.fields[0].candidates.len(), 1);
        assert_eq!(assignment.fields[0].candidates[0].raw_value, "+15550101");
        // Each phone-like token also surfaces an integer candidate (evidence,
        // not assignments); together with the unmatched "Fax" phone they stay
        // observable as unassigned.
        let unassigned_phones = assignment
            .unassigned_candidates
            .iter()
            .filter(|candidate| candidate.candidate_type == CandidateType::PhoneNumber)
            .count();
        assert_eq!(assignment.unassigned_candidates.len(), 3);
        assert_eq!(unassigned_phones, 1);
        assert_eq!(assignment.unassigned_candidates[0].source_column, Some(1));
    }

    #[test]
    fn blocks_without_row_provenance_are_reported_not_dropped() {
        let document = table_document(vec![
            table_block(None, 1, 1, RawValue::text("Email")),
            table_block(None, 1, 2, RawValue::text("Age")),
            table_block(None, 2, 1, RawValue::text("ada@example.test")),
            table_block(None, 2, 2, RawValue::Integer(30)),
            RawBlock {
                id: "loose-block".to_owned(),
                value: RawValue::text("stray note"),
                location: SourceLocation::default(),
            },
        ]);

        let grouped = group_document_rows(&document);

        assert_eq!(grouped.rows.len(), 2);
        assert_eq!(grouped.warnings.len(), 1);
        assert_eq!(grouped.warnings[0].code, "row_provenance_missing");
        assert_eq!(
            grouped.warnings[0].location.as_ref().and_then(|l| l.row),
            None
        );
    }

    #[test]
    fn sheets_get_independent_header_contexts() {
        let document = table_document(vec![
            table_block(Some("A"), 1, 1, RawValue::text("Email")),
            table_block(Some("A"), 1, 2, RawValue::text("Age")),
            table_block(Some("A"), 2, 1, RawValue::text("ada@example.test")),
            table_block(Some("A"), 2, 2, RawValue::Integer(30)),
            table_block(Some("B"), 1, 1, RawValue::text("Phone")),
            table_block(Some("B"), 1, 2, RawValue::text("Fax")),
            table_block(Some("B"), 2, 1, RawValue::text("+15550101")),
            table_block(Some("B"), 2, 2, RawValue::text("+15550102")),
        ]);
        let fields = [
            field_of("email", &[], CandidateType::Email, false, false),
            field_of("phone", &[], CandidateType::PhoneNumber, false, false),
        ];

        let result = parse_document_rows_with_assignment(&document, &fields, &[]);

        assert_eq!(result.sheets.len(), 2);
        assert_eq!(result.sheets[0].sheet.as_deref(), Some("A"));
        assert_eq!(result.sheets[1].sheet.as_deref(), Some("B"));
        for sheet in &result.sheets {
            assert!(sheet.header.context().is_some());
            assert_eq!(sheet.records.len(), 1);
        }
        let assigned_value = |sheet: &SheetTableResult, name: &str| {
            sheet.records[0]
                .parse
                .assignment
                .fields
                .iter()
                .find(|field| field.name == name)
                .map(|field| field.candidates[0].raw_value.clone())
        };
        // Sheets only report fields that found a compatible candidate.
        assert_eq!(
            assigned_value(&result.sheets[0], "email").as_deref(),
            Some("ada@example.test")
        );
        assert_eq!(assigned_value(&result.sheets[0], "phone"), None);
        assert_eq!(
            assigned_value(&result.sheets[1], "phone").as_deref(),
            Some("+15550101")
        );
        assert_eq!(assigned_value(&result.sheets[1], "email"), None);
    }

    #[test]
    fn table_parse_result_round_trips_as_json() {
        let document = table_document(vec![
            table_block(None, 1, 1, RawValue::text("Email")),
            table_block(None, 1, 2, RawValue::text("Age")),
            table_block(None, 2, 1, RawValue::text("ada@example.test")),
            table_block(None, 2, 2, RawValue::Integer(30)),
        ]);
        let fields = [field_of("email", &[], CandidateType::Email, true, false)];

        let result = parse_document_rows_with_assignment(&document, &fields, &[]);
        let json = serde_json::to_string(&result).expect("table result should serialize");
        let decoded: TableParseResult =
            serde_json::from_str(&json).expect("table result should deserialize");

        assert_eq!(decoded, result);
    }
}

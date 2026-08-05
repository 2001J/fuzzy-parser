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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
}

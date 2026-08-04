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
    Null,
}

impl RawValue {
    pub fn text(value: impl Into<String>) -> Self {
        Self::Text(value.into())
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

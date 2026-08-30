//! Versioned public failures. In-process causes and explicit diagnostics may be sensitive.
use crate::{IoErrorKind, ParserError};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Independent of the parse-response, schema and package versions.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ErrorContractVersion {
    #[serde(rename = "0.1")]
    V0_1,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticsMode {
    #[default]
    Safe,
    /// Caller context can contain private data. Do not send it to public logs.
    Detailed,
}

/// An allowlist, not a place for raw input, full schemas or dependency error prose.
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sheet: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SchemaValidationReason {
    EmptySchemaVersion,
    UnsupportedSchemaVersion,
    EmptyFieldName,
    DuplicateFieldName,
    DuplicateFieldLabel,
    EmptyAlias,
    EmptyEnumValue,
    DuplicateEnumValue,
    EmptyEnumAlias,
    DuplicateEnumAlias,
    InvalidIntegerRange,
    InvalidLengthRange,
}

impl fmt::Display for SchemaValidationReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::EmptySchemaVersion => "schema version must not be empty",
            Self::UnsupportedSchemaVersion => "unsupported schema version",
            Self::EmptyFieldName => "field name must not be empty",
            Self::DuplicateFieldName => "duplicate field name",
            Self::DuplicateFieldLabel => "duplicate field label",
            Self::EmptyAlias => "field has an empty alias",
            Self::EmptyEnumValue => "enum field has an empty value",
            Self::DuplicateEnumValue => "enum field repeats a value",
            Self::EmptyEnumAlias => "enum value has an empty alias",
            Self::DuplicateEnumAlias => "enum field repeats an alias",
            Self::InvalidIntegerRange => "field has an invalid integer range",
            Self::InvalidLengthRange => "field has an invalid length range",
        })
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UnsupportedFieldType {
    Text,
    PersonName,
    Datetime,
}

impl fmt::Display for UnsupportedFieldType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Text => "text",
            Self::PersonName => "person_name",
            Self::Datetime => "datetime",
        })
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SchemaFailureCause {
    Json,
    Validation { reason: SchemaValidationReason },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OutputTarget {
    ParseResult,
    RawDocument,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TableSelectionReason {
    UnsupportedSource,
    EmptySheetSelection,
    DuplicateSheetSelection,
    MissingSheet,
    SheetIndexOutOfRange,
    InvalidRowRange,
    OverlappingRowRange,
    RowNotFound,
    HeaderNotFound,
    HeaderConflict,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResourceLimitKind {
    CsvBytes,
    CsvRows,
    CsvCells,
    XlsxBytes,
    XlsxSheets,
    XlsxCells,
    SchemaBytes,
    SchemaFields,
    SchemaAliases,
    SchemaNesting,
    Records,
    ResponseBytes,
}

/// Safe metadata only. User strings belong in explicitly requested diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "code")]
pub enum FailureKind {
    #[serde(rename = "io_error")]
    Io { kind: IoErrorKind },
    #[serde(rename = "invalid_utf8")]
    InvalidUtf8 { valid_up_to: usize },
    #[serde(rename = "unsupported_input")]
    UnsupportedInput,
    #[serde(rename = "not_regular_file")]
    NotRegularFile,
    #[serde(rename = "empty_input")]
    EmptyInput,
    #[serde(rename = "file_too_large")]
    FileTooLarge { limit: u64, actual: u64 },
    #[serde(rename = "input_too_large")]
    InputTooLarge { limit: usize, actual: usize },
    #[serde(rename = "line_too_long")]
    LineTooLong {
        line: usize,
        limit: usize,
        actual: usize,
    },
    #[serde(rename = "invalid_csv")]
    InvalidCsv { record: Option<usize> },
    #[serde(rename = "invalid_xlsx")]
    InvalidXlsx,
    #[serde(rename = "schema_io_error")]
    SchemaIo { kind: IoErrorKind },
    #[serde(rename = "schema_input_error")]
    SchemaInput,
    #[serde(rename = "schema_parse_error")]
    SchemaParse,
    #[serde(rename = "schema_validation_error")]
    SchemaValidation { reason: SchemaValidationReason },
    #[serde(rename = "schema_field_type_unsupported")]
    SchemaFieldTypeUnsupported { field_type: UnsupportedFieldType },
    #[serde(rename = "schema_option_unsupported")]
    SchemaOptionUnsupported,
    #[serde(rename = "schema_constraint_unsupported")]
    SchemaConstraintUnsupported,
    #[serde(rename = "schema_property_unsupported")]
    SchemaPropertyUnsupported,
    #[serde(rename = "schema_enum_definition_ambiguous")]
    SchemaEnumDefinitionAmbiguous,
    #[serde(rename = "schema_enum_definition_unsupported")]
    SchemaEnumDefinitionUnsupported,
    #[serde(rename = "schema_serialization_error")]
    SchemaSerialization { cause: SchemaFailureCause },
    #[serde(rename = "output_serialization_error")]
    OutputSerialization { target: OutputTarget },
    #[serde(rename = "table_selection_error")]
    TableSelection { reason: TableSelectionReason },
    #[serde(rename = "resource_limit")]
    ResourceLimit {
        resource: ResourceLimitKind,
        limit: u64,
        actual: u64,
    },
}

impl fmt::Display for FailureKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { kind } => write!(f, "could not read input: {kind}"),
            Self::InvalidUtf8 { valid_up_to } => {
                write!(f, "input is not valid UTF-8 at byte offset {valid_up_to}")
            }
            Self::UnsupportedInput => f.write_str("unsupported input type"),
            Self::NotRegularFile => f.write_str("input is not a regular file"),
            Self::EmptyInput => f.write_str("empty input is not allowed"),
            Self::FileTooLarge { limit, actual } => {
                write!(f, "file exceeds the {limit}-byte limit ({actual} bytes)")
            }
            Self::InputTooLarge { limit, actual } => {
                write!(f, "input exceeds the {limit}-byte limit ({actual} bytes)")
            }
            Self::LineTooLong {
                line,
                limit,
                actual,
            } => write!(
                f,
                "input line {line} exceeds the {limit}-byte limit ({actual} bytes)"
            ),
            Self::InvalidCsv {
                record: Some(record),
            } => write!(f, "invalid CSV input at record {record}"),
            Self::InvalidCsv { record: None } => f.write_str("invalid CSV input"),
            Self::InvalidXlsx => f.write_str("could not read XLSX workbook"),
            Self::SchemaIo { kind } => write!(f, "could not read schema: {kind}"),
            Self::SchemaInput => f.write_str("schema text must be valid UTF-8"),
            Self::SchemaParse => f.write_str("invalid schema JSON"),
            Self::SchemaValidation { reason } => write!(f, "invalid schema: {reason}"),
            Self::SchemaFieldTypeUnsupported { field_type } => write!(
                f,
                "field type \"{field_type}\" is not supported by the parser yet"
            ),
            Self::SchemaSerialization { .. } => f.write_str("could not serialize schema"),
            Self::SchemaOptionUnsupported => {
                f.write_str("schema option is not supported for execution")
            }
            Self::SchemaConstraintUnsupported => {
                f.write_str("schema constraint is not supported for this field type")
            }
            Self::SchemaPropertyUnsupported => {
                f.write_str("schema property is not supported for execution")
            }
            Self::SchemaEnumDefinitionAmbiguous => {
                f.write_str("enum field has ambiguous lexical definitions")
            }
            Self::SchemaEnumDefinitionUnsupported => {
                f.write_str("enum definition cannot be detected by the current parser")
            }
            Self::OutputSerialization {
                target: OutputTarget::ParseResult,
            } => f.write_str("could not serialize parse result"),
            Self::OutputSerialization {
                target: OutputTarget::RawDocument,
            } => f.write_str("could not serialize raw document"),
            Self::TableSelection { reason } => f.write_str(match reason {
                TableSelectionReason::UnsupportedSource => {
                    "table selection is not supported for this source"
                }
                TableSelectionReason::EmptySheetSelection => "sheet selection must not be empty",
                TableSelectionReason::DuplicateSheetSelection => {
                    "the same sheet was selected more than once"
                }
                TableSelectionReason::MissingSheet => "selected sheet was not found",
                TableSelectionReason::SheetIndexOutOfRange => {
                    "selected sheet index is out of range"
                }
                TableSelectionReason::InvalidRowRange => "row range is invalid",
                TableSelectionReason::OverlappingRowRange => "row ranges overlap",
                TableSelectionReason::RowNotFound => "selected row was not found",
                TableSelectionReason::HeaderNotFound => "selected header row was not found",
                TableSelectionReason::HeaderConflict => {
                    "header selection conflicts with row selection"
                }
            }),
            Self::ResourceLimit {
                resource,
                limit,
                actual,
            } => {
                write!(f, "{resource:?} exceeds the {limit}-unit limit ({actual})")
            }
        }
    }
}

/// Exact new-wire round-trip boundary. Missing/unknown error versions are rejected.
/// This is not a lossless serialization of the private in-process cause.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ErrorPayload {
    pub error_contract_version: ErrorContractVersion,
    #[serde(flatten)]
    pub failure: FailureKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<DiagnosticContext>,
}

impl fmt::Display for ErrorPayload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.failure)?;
        if let Some(context) = &self.diagnostics {
            // JSON escaping prevents caller control characters becoming terminal controls.
            let json = serde_json::to_string(context).map_err(|_| fmt::Error)?;
            write!(f, " [diagnostics: {json}]")?;
        }
        Ok(())
    }
}

/// The payload is authoritative. Incoming outer messages are ignored, never stored.
/// Serialize, Display and message() always render the current typed payload.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct ErrorReport {
    pub error: ErrorPayload,
}

impl ErrorReport {
    pub fn message(&self) -> String {
        self.error.to_string()
    }
}

impl Serialize for ErrorReport {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut report = serializer.serialize_struct("ErrorReport", 2)?;
        report.serialize_field("error", &self.error)?;
        report.serialize_field("message", &self.message())?;
        report.end()
    }
}

impl fmt::Display for ErrorReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(f)
    }
}

/// Shared in-process failure. Debug/context may be sensitive; Display/Serialize default safe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    pub kind: FailureKind,
    context: Option<Box<DiagnosticContext>>,
}

impl Failure {
    pub fn new(kind: FailureKind) -> Self {
        Self {
            kind,
            context: None,
        }
    }

    pub fn with_context(mut self, context: DiagnosticContext) -> Self {
        if context != DiagnosticContext::default() {
            self.context = Some(Box::new(context));
        }
        self
    }

    pub fn payload(&self, mode: DiagnosticsMode) -> ErrorPayload {
        ErrorPayload {
            error_contract_version: ErrorContractVersion::V0_1,
            failure: self.kind.clone(),
            diagnostics: match mode {
                DiagnosticsMode::Safe => None,
                DiagnosticsMode::Detailed => self.context.as_deref().cloned(),
            },
        }
    }

    pub fn with_path(mut self, path: &str) -> Self {
        let mut context = self
            .context
            .take()
            .map(|context| *context)
            .unwrap_or_default();
        context.path = nonempty(path);
        self.with_context(context)
    }

    pub fn report(&self, mode: DiagnosticsMode) -> ErrorReport {
        ErrorReport {
            error: self.payload(mode),
        }
    }
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.kind.fmt(f)
    }
}

impl std::error::Error for Failure {}

impl Serialize for Failure {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.payload(DiagnosticsMode::Safe).serialize(serializer)
    }
}

impl From<&ParserError> for Failure {
    fn from(error: &ParserError) -> Self {
        let mut context = DiagnosticContext::default();
        let kind = match error {
            ParserError::Io { path, kind } => {
                context.path = nonempty(path);
                FailureKind::Io { kind: kind.clone() }
            }
            ParserError::InvalidUtf8 { path, valid_up_to } => {
                context.path = nonempty(path);
                FailureKind::InvalidUtf8 {
                    valid_up_to: *valid_up_to,
                }
            }
            ParserError::UnsupportedInput { source_type } => {
                context.source_type = Some(source_type.clone());
                FailureKind::UnsupportedInput
            }
            ParserError::NotRegularFile { path } => {
                context.path = nonempty(path);
                FailureKind::NotRegularFile
            }
            ParserError::EmptyInput { path } => {
                context.path = nonempty(path);
                FailureKind::EmptyInput
            }
            ParserError::FileTooLarge {
                path,
                limit,
                actual,
            } => {
                context.path = nonempty(path);
                FailureKind::FileTooLarge {
                    limit: *limit,
                    actual: *actual,
                }
            }
            ParserError::InputTooLarge {
                source,
                limit,
                actual,
            } => {
                context.source = Some(source.clone());
                FailureKind::InputTooLarge {
                    limit: *limit,
                    actual: *actual,
                }
            }
            ParserError::LineTooLong {
                source,
                line,
                limit,
                actual,
            } => {
                context.source = Some(source.clone());
                FailureKind::LineTooLong {
                    line: *line,
                    limit: *limit,
                    actual: *actual,
                }
            }
            ParserError::InvalidCsv { path, record, .. } => {
                context.path = nonempty(path);
                FailureKind::InvalidCsv { record: *record }
            }
            ParserError::InvalidXlsx { path, .. } => {
                context.path = nonempty(path);
                FailureKind::InvalidXlsx
            }
            ParserError::ResourceLimit {
                resource,
                limit,
                actual,
            } => FailureKind::ResourceLimit {
                resource: *resource,
                limit: *limit,
                actual: *actual,
            },
        };
        Self::new(kind).with_context(context)
    }
}

fn nonempty(value: &str) -> Option<String> {
    (!value.is_empty()).then(|| value.to_owned())
}

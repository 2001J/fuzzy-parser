use parser_core::{
    DiagnosticContext, DiagnosticsMode, ErrorReport, Failure, FailureKind, SchemaFailureCause,
    SchemaValidationReason,
};
use serde::{Deserialize, Serialize};
use std::{error::Error, fmt};

pub const SCHEMA_VERSION: &str = "0.1";

mod compile;
pub use compile::{compile_schema, compile_schema_json, decode_execution_schema};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchemaLimits {
    pub max_bytes: usize,
    pub max_fields: usize,
    pub max_aliases: usize,
    pub max_nesting: usize,
}
impl Default for SchemaLimits {
    fn default() -> Self {
        Self {
            max_bytes: 1024 * 1024,
            max_fields: 1024,
            max_aliases: 8192,
            max_nesting: 32,
        }
    }
}

pub fn decode_execution_schema_with_limits(
    input: &str,
    limits: SchemaLimits,
) -> Result<TargetSchema, Failure> {
    if input.len() > limits.max_bytes {
        return Err(Failure::new(FailureKind::ResourceLimit {
            resource: parser_core::ResourceLimitKind::SchemaBytes,
            limit: limits.max_bytes as u64,
            actual: input.len() as u64,
        }));
    }
    let value: serde_json::Value =
        serde_json::from_str(input).map_err(|_| Failure::new(FailureKind::SchemaParse))?;
    let fields = value
        .get("fields")
        .and_then(|v| v.as_array())
        .map_or(0, Vec::len);
    if fields > limits.max_fields {
        return Err(Failure::new(FailureKind::ResourceLimit {
            resource: parser_core::ResourceLimitKind::SchemaFields,
            limit: limits.max_fields as u64,
            actual: fields as u64,
        }));
    }
    let aliases = value
        .get("fields")
        .and_then(|v| v.as_array())
        .map_or(0, |fs| {
            fs.iter()
                .map(|f| {
                    f.get("aliases")
                        .and_then(|a| a.as_array())
                        .map_or(0, Vec::len)
                })
                .sum()
        });
    if aliases > limits.max_aliases {
        return Err(Failure::new(FailureKind::ResourceLimit {
            resource: parser_core::ResourceLimitKind::SchemaAliases,
            limit: limits.max_aliases as u64,
            actual: aliases as u64,
        }));
    }
    decode_execution_schema(input)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TargetSchema {
    pub schema_version: String,
    pub record_name: Option<String>,
    pub fields: Vec<FieldDefinition>,
    pub options: SchemaOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SchemaOptions {
    pub allow_unknown_fields: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_pipeline: Option<TextPipelineOptions>,
}

impl Default for SchemaOptions {
    fn default() -> Self {
        Self {
            allow_unknown_fields: true,
            text_pipeline: None,
        }
    }
}

/// Serialized caller configuration for opt-in text composition.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TextPipelineOptions {
    pub normalization: TextNormalizationOptions,
    pub strategy: TextSegmentationStrategy,
    #[serde(default)]
    pub repeated_identifier_markers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TextNormalizationOptions {
    pub normalize_line_endings: bool,
    pub trim_whitespace: bool,
    pub collapse_whitespace: bool,
    pub normalize_punctuation: bool,
    pub mark_noise: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TextSegmentationStrategy {
    OneBlockPerRecord,
    JoinIndentedContinuations,
    SplitRepeatedIdentifiers,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FieldDefinition {
    pub name: String,
    pub field_type: FieldType,
    pub required: bool,
    pub multiple: bool,
    pub aliases: Vec<String>,
    pub constraints: Vec<FieldConstraint>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FieldType {
    Text,
    PersonName,
    PhoneNumber,
    Email,
    Integer,
    Decimal,
    Currency,
    Date,
    Datetime,
    Boolean,
    Enum { values: Vec<EnumValue> },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EnumValue {
    pub value: String,
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum FieldConstraint {
    MinimumInteger(i64),
    MaximumInteger(i64),
    MinimumLength(usize),
    MaximumLength(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaValidationError {
    EmptySchemaVersion,
    UnsupportedSchemaVersion(String),
    EmptyFieldName,
    DuplicateFieldName(String),
    DuplicateFieldLabel(String),
    EmptyAlias { field: String },
    EmptyEnumValue { field: String },
    DuplicateEnumValue { field: String, value: String },
    EmptyEnumAlias { field: String, value: String },
    DuplicateEnumAlias { field: String, alias: String },
    InvalidIntegerRange { field: String },
    InvalidLengthRange { field: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaParseError {
    InvalidJson(String),
    InvalidSchema(SchemaValidationError),
}

impl fmt::Display for SchemaParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        Failure::from(self).fmt(formatter)
    }
}

impl Error for SchemaParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidSchema(error) => Some(error),
            Self::InvalidJson(_) => None,
        }
    }
}

impl fmt::Display for SchemaValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        Failure::from(self).fmt(formatter)
    }
}

impl Error for SchemaValidationError {}

impl From<&SchemaValidationError> for Failure {
    fn from(error: &SchemaValidationError) -> Self {
        let mut context = DiagnosticContext::default();
        let reason = match error {
            SchemaValidationError::EmptySchemaVersion => SchemaValidationReason::EmptySchemaVersion,
            SchemaValidationError::UnsupportedSchemaVersion(version) => {
                context.version = Some(version.clone());
                SchemaValidationReason::UnsupportedSchemaVersion
            }
            SchemaValidationError::EmptyFieldName => SchemaValidationReason::EmptyFieldName,
            SchemaValidationError::DuplicateFieldName(field) => {
                context.field = Some(field.clone());
                SchemaValidationReason::DuplicateFieldName
            }
            SchemaValidationError::DuplicateFieldLabel(value) => {
                context.value = Some(value.clone());
                SchemaValidationReason::DuplicateFieldLabel
            }
            SchemaValidationError::EmptyAlias { field } => {
                context.field = Some(field.clone());
                SchemaValidationReason::EmptyAlias
            }
            SchemaValidationError::EmptyEnumValue { field } => {
                context.field = Some(field.clone());
                SchemaValidationReason::EmptyEnumValue
            }
            SchemaValidationError::DuplicateEnumValue { field, value } => {
                context.field = Some(field.clone());
                context.value = Some(value.clone());
                SchemaValidationReason::DuplicateEnumValue
            }
            SchemaValidationError::EmptyEnumAlias { field, value } => {
                context.field = Some(field.clone());
                context.value = Some(value.clone());
                SchemaValidationReason::EmptyEnumAlias
            }
            SchemaValidationError::DuplicateEnumAlias { field, alias } => {
                context.field = Some(field.clone());
                context.alias = Some(alias.clone());
                SchemaValidationReason::DuplicateEnumAlias
            }
            SchemaValidationError::InvalidIntegerRange { field } => {
                context.field = Some(field.clone());
                SchemaValidationReason::InvalidIntegerRange
            }
            SchemaValidationError::InvalidLengthRange { field } => {
                context.field = Some(field.clone());
                SchemaValidationReason::InvalidLengthRange
            }
        };
        Failure::new(FailureKind::SchemaValidation { reason }).with_context(context)
    }
}

impl From<&SchemaParseError> for Failure {
    fn from(error: &SchemaParseError) -> Self {
        match error {
            SchemaParseError::InvalidJson(_) => Failure::new(FailureKind::SchemaParse),
            SchemaParseError::InvalidSchema(error) => Failure::from(error),
        }
    }
}

impl SchemaParseError {
    pub fn report(&self, mode: DiagnosticsMode) -> ErrorReport {
        Failure::from(self).report(mode)
    }

    pub fn serialization_failure(&self) -> Failure {
        let mut failure = Failure::from(self);
        let cause = match failure.kind {
            FailureKind::SchemaValidation { reason } => SchemaFailureCause::Validation { reason },
            _ => SchemaFailureCause::Json,
        };
        failure.kind = FailureKind::SchemaSerialization { cause };
        failure
    }
}

impl SchemaValidationError {
    pub fn report(&self, mode: DiagnosticsMode) -> ErrorReport {
        Failure::from(self).report(mode)
    }
}

impl TargetSchema {
    pub fn from_json(input: &str) -> Result<Self, SchemaParseError> {
        let schema: TargetSchema = serde_json::from_str(input)
            .map_err(|error| SchemaParseError::InvalidJson(error.to_string()))?;
        schema.validate().map_err(SchemaParseError::InvalidSchema)?;
        Ok(schema)
    }

    pub fn to_json(&self) -> Result<String, SchemaParseError> {
        self.validate().map_err(SchemaParseError::InvalidSchema)?;
        serde_json::to_string_pretty(self)
            .map_err(|error| SchemaParseError::InvalidJson(error.to_string()))
    }

    pub fn validate(&self) -> Result<(), SchemaValidationError> {
        if self.schema_version.trim().is_empty() {
            return Err(SchemaValidationError::EmptySchemaVersion);
        }
        if self.schema_version != SCHEMA_VERSION {
            return Err(SchemaValidationError::UnsupportedSchemaVersion(
                self.schema_version.clone(),
            ));
        }

        let mut field_names = Vec::new();
        let mut field_labels = Vec::new();
        for field in &self.fields {
            if field.name.trim().is_empty() {
                return Err(SchemaValidationError::EmptyFieldName);
            }
            if field_names.iter().any(|name| name == &field.name) {
                return Err(SchemaValidationError::DuplicateFieldName(
                    field.name.clone(),
                ));
            }
            field_names.push(field.name.clone());

            let field_label = field.name.to_ascii_lowercase();
            if field_labels.iter().any(|label| label == &field_label) {
                return Err(SchemaValidationError::DuplicateFieldLabel(
                    field.name.clone(),
                ));
            }
            field_labels.push(field_label);

            if field.aliases.iter().any(|alias| alias.trim().is_empty()) {
                return Err(SchemaValidationError::EmptyAlias {
                    field: field.name.clone(),
                });
            }
            for alias in &field.aliases {
                let label = alias.to_ascii_lowercase();
                if field_labels.iter().any(|existing| existing == &label) {
                    return Err(SchemaValidationError::DuplicateFieldLabel(alias.clone()));
                }
                field_labels.push(label);
            }
            validate_constraints(field)?;
            if let FieldType::Enum { values } = &field.field_type {
                validate_enum_values(&field.name, values)?;
            }
        }
        Ok(())
    }
}

fn validate_constraints(field: &FieldDefinition) -> Result<(), SchemaValidationError> {
    let minimum_integer = field
        .constraints
        .iter()
        .find_map(|constraint| match constraint {
            FieldConstraint::MinimumInteger(value) => Some(*value),
            _ => None,
        });
    let maximum_integer = field
        .constraints
        .iter()
        .find_map(|constraint| match constraint {
            FieldConstraint::MaximumInteger(value) => Some(*value),
            _ => None,
        });
    if minimum_integer
        .zip(maximum_integer)
        .is_some_and(|(minimum, maximum)| minimum > maximum)
    {
        return Err(SchemaValidationError::InvalidIntegerRange {
            field: field.name.clone(),
        });
    }

    let minimum_length = field
        .constraints
        .iter()
        .find_map(|constraint| match constraint {
            FieldConstraint::MinimumLength(value) => Some(*value),
            _ => None,
        });
    let maximum_length = field
        .constraints
        .iter()
        .find_map(|constraint| match constraint {
            FieldConstraint::MaximumLength(value) => Some(*value),
            _ => None,
        });
    if minimum_length
        .zip(maximum_length)
        .is_some_and(|(minimum, maximum)| minimum > maximum)
    {
        return Err(SchemaValidationError::InvalidLengthRange {
            field: field.name.clone(),
        });
    }
    Ok(())
}

fn validate_enum_values(field: &str, values: &[EnumValue]) -> Result<(), SchemaValidationError> {
    let mut seen_values = Vec::new();
    let mut seen_aliases = Vec::new();
    for enum_value in values {
        if enum_value.value.trim().is_empty() {
            return Err(SchemaValidationError::EmptyEnumValue {
                field: field.to_owned(),
            });
        }
        if seen_values.iter().any(|value| value == &enum_value.value) {
            return Err(SchemaValidationError::DuplicateEnumValue {
                field: field.to_owned(),
                value: enum_value.value.clone(),
            });
        }
        seen_values.push(enum_value.value.clone());
        if enum_value
            .aliases
            .iter()
            .any(|alias| alias.trim().is_empty())
        {
            return Err(SchemaValidationError::EmptyEnumAlias {
                field: field.to_owned(),
                value: enum_value.value.clone(),
            });
        }
        for alias in &enum_value.aliases {
            let normalized_alias = alias.to_ascii_lowercase();
            if seen_aliases.iter().any(|value| value == &normalized_alias)
                || seen_values
                    .iter()
                    .any(|value| value.to_ascii_lowercase() == normalized_alias)
            {
                return Err(SchemaValidationError::DuplicateEnumAlias {
                    field: field.to_owned(),
                    alias: alias.clone(),
                });
            }
            seen_aliases.push(normalized_alias);
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;

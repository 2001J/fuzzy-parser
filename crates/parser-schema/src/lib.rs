use serde::{Deserialize, Serialize};
use std::{error::Error, fmt};

pub const SCHEMA_VERSION: &str = "0.1";

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
}

impl Default for SchemaOptions {
    fn default() -> Self {
        Self {
            allow_unknown_fields: true,
        }
    }
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
    EmptyAlias { field: String },
    EmptyEnumValue { field: String },
    DuplicateEnumValue { field: String, value: String },
    EmptyEnumAlias { field: String, value: String },
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
        match self {
            Self::InvalidJson(message) => write!(formatter, "invalid schema JSON: {message}"),
            Self::InvalidSchema(error) => write!(formatter, "invalid schema: {error}"),
        }
    }
}

impl Error for SchemaParseError {}

impl fmt::Display for SchemaValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptySchemaVersion => write!(formatter, "schema version must not be empty"),
            Self::UnsupportedSchemaVersion(version) => {
                write!(formatter, "unsupported schema version: {version}")
            }
            Self::EmptyFieldName => write!(formatter, "field name must not be empty"),
            Self::DuplicateFieldName(name) => write!(formatter, "duplicate field name: {name}"),
            Self::EmptyAlias { field } => write!(formatter, "field {field} has an empty alias"),
            Self::EmptyEnumValue { field } => {
                write!(formatter, "enum field {field} has an empty value")
            }
            Self::DuplicateEnumValue { field, value } => {
                write!(formatter, "enum field {field} repeats value: {value}")
            }
            Self::EmptyEnumAlias { field, value } => {
                write!(
                    formatter,
                    "enum value {value} in field {field} has an empty alias"
                )
            }
            Self::InvalidIntegerRange { field } => {
                write!(formatter, "field {field} has an invalid integer range")
            }
            Self::InvalidLengthRange { field } => {
                write!(formatter, "field {field} has an invalid length range")
            }
        }
    }
}

impl Error for SchemaValidationError {}

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

            if field.aliases.iter().any(|alias| alias.trim().is_empty()) {
                return Err(SchemaValidationError::EmptyAlias {
                    field: field.name.clone(),
                });
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
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_schema_passes_validation() {
        let schema = TargetSchema {
            schema_version: SCHEMA_VERSION.to_owned(),
            record_name: Some("contact".to_owned()),
            fields: vec![FieldDefinition {
                name: "status".to_owned(),
                field_type: FieldType::Enum {
                    values: vec![EnumValue {
                        value: "active".to_owned(),
                        aliases: vec!["enabled".to_owned()],
                    }],
                },
                required: true,
                multiple: false,
                aliases: vec!["state".to_owned()],
                constraints: Vec::new(),
            }],
            options: SchemaOptions::default(),
        };

        assert_eq!(schema.validate(), Ok(()));
    }

    #[test]
    fn duplicate_field_names_are_rejected() {
        let field = FieldDefinition {
            name: "email".to_owned(),
            field_type: FieldType::Email,
            required: false,
            multiple: false,
            aliases: Vec::new(),
            constraints: Vec::new(),
        };
        let schema = TargetSchema {
            schema_version: SCHEMA_VERSION.to_owned(),
            record_name: None,
            fields: vec![field.clone(), field],
            options: SchemaOptions::default(),
        };

        assert_eq!(
            schema.validate(),
            Err(SchemaValidationError::DuplicateFieldName(
                "email".to_owned()
            ))
        );
    }

    #[test]
    fn invalid_enum_values_are_rejected() {
        let schema = TargetSchema {
            schema_version: SCHEMA_VERSION.to_owned(),
            record_name: None,
            fields: vec![FieldDefinition {
                name: "status".to_owned(),
                field_type: FieldType::Enum {
                    values: vec![EnumValue {
                        value: "active".to_owned(),
                        aliases: vec![" ".to_owned()],
                    }],
                },
                required: false,
                multiple: false,
                aliases: Vec::new(),
                constraints: Vec::new(),
            }],
            options: SchemaOptions::default(),
        };

        assert_eq!(
            schema.validate(),
            Err(SchemaValidationError::EmptyEnumAlias {
                field: "status".to_owned(),
                value: "active".to_owned(),
            })
        );
    }

    #[test]
    fn unsupported_schema_versions_are_rejected() {
        let schema = TargetSchema {
            schema_version: "9.9".to_owned(),
            record_name: None,
            fields: Vec::new(),
            options: SchemaOptions::default(),
        };

        assert_eq!(
            schema.validate(),
            Err(SchemaValidationError::UnsupportedSchemaVersion(
                "9.9".to_owned()
            ))
        );
    }

    #[test]
    fn valid_schema_round_trips_through_json() {
        let schema = TargetSchema {
            schema_version: SCHEMA_VERSION.to_owned(),
            record_name: Some("contact".to_owned()),
            fields: Vec::new(),
            options: SchemaOptions::default(),
        };
        let json = schema.to_json().expect("schema should serialize");

        assert_eq!(TargetSchema::from_json(&json), Ok(schema));
    }

    #[test]
    fn invalid_schema_json_reports_validation_error() {
        let json = r#"{"schema_version":"0.1","record_name":null,"fields":[{"name":"","field_type":"email","required":false,"multiple":false,"aliases":[],"constraints":[]}],"options":{"allow_unknown_fields":true}}"#;

        assert_eq!(
            TargetSchema::from_json(json),
            Err(SchemaParseError::InvalidSchema(
                SchemaValidationError::EmptyFieldName
            ))
        );
    }
}

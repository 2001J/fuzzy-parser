use crate::TargetSchema;
use parser_core::{DiagnosticContext, Failure, FailureKind, UnsupportedFieldType};

/// Validate and compile a typed schema. JSON callers should use
/// `compile_schema_json` so unknown properties cannot disappear before checking.
pub fn compile_schema(schema: &TargetSchema) -> Result<parser_core::ParsePlan, Failure> {
    use crate::{FieldConstraint, FieldType};
    use parser_core::{AssignmentConstraint, AssignmentField, CandidateType};
    schema.validate().map_err(|error| Failure::from(&error))?;
    // Preserve the old compiler's first unsupported-type failure before new
    // option, enum-definition or constraint capability checks.
    let types = schema
        .fields
        .iter()
        .map(candidate_type)
        .collect::<Result<Vec<_>, _>>()?;
    if !schema.options.allow_unknown_fields {
        return Err(Failure::new(FailureKind::SchemaOptionUnsupported));
    }

    let mut fields = Vec::new();

    for (field, candidate_type) in schema.fields.iter().zip(types) {
        let mut enum_definitions = Vec::new();
        if let FieldType::Enum { values } = &field.field_type {
            validate_executable_enum(&field.name, values)?;
            for value in values {
                enum_definitions.push((value.value.clone(), value.aliases.clone()));
            }
        }

        for constraint in &field.constraints {
            let applicable = match constraint {
                FieldConstraint::MinimumInteger(_) | FieldConstraint::MaximumInteger(_) => {
                    candidate_type == CandidateType::Integer
                }
                FieldConstraint::MinimumLength(_) | FieldConstraint::MaximumLength(_) => {
                    matches!(
                        candidate_type,
                        CandidateType::Email
                            | CandidateType::PhoneNumber
                            | CandidateType::Date
                            | CandidateType::Enum
                    )
                }
            };
            if !applicable {
                return Err(field_failure(
                    FailureKind::SchemaConstraintUnsupported,
                    &field.name,
                ));
            }
        }
        let constraints = field
            .constraints
            .iter()
            .map(|constraint| match constraint {
                FieldConstraint::MinimumInteger(value) => {
                    AssignmentConstraint::MinimumInteger(*value)
                }
                FieldConstraint::MaximumInteger(value) => {
                    AssignmentConstraint::MaximumInteger(*value)
                }
                FieldConstraint::MinimumLength(value) => {
                    AssignmentConstraint::MinimumLength(*value)
                }
                FieldConstraint::MaximumLength(value) => {
                    AssignmentConstraint::MaximumLength(*value)
                }
            })
            .collect();

        fields.push(parser_core::PlanField::new(
            AssignmentField {
                name: field.name.clone(),
                aliases: field.aliases.clone(),
                candidate_type,
                required: field.required,
                multiple: field.multiple,
                unique: false,
                constraints,
                expected_column: None,
            },
            enum_definitions,
        ));
    }

    Ok(parser_core::ParsePlan::new(
        fields,
        schema.record_name.clone(),
    ))
}

fn candidate_type(field: &crate::FieldDefinition) -> Result<parser_core::CandidateType, Failure> {
    use crate::FieldType;
    use parser_core::CandidateType;
    let field_type = match &field.field_type {
        FieldType::Email => return Ok(CandidateType::Email),
        FieldType::Integer => return Ok(CandidateType::Integer),
        FieldType::Decimal => return Ok(CandidateType::Decimal),
        FieldType::PhoneNumber => return Ok(CandidateType::PhoneNumber),
        FieldType::Boolean => return Ok(CandidateType::Boolean),
        FieldType::Date => return Ok(CandidateType::Date),
        FieldType::Currency => return Ok(CandidateType::Currency),
        FieldType::Enum { .. } => return Ok(CandidateType::Enum),
        FieldType::Datetime => UnsupportedFieldType::Datetime,
        FieldType::Text => UnsupportedFieldType::Text,
        FieldType::PersonName => UnsupportedFieldType::PersonName,
    };
    Err(field_failure(
        FailureKind::SchemaFieldTypeUnsupported { field_type },
        &field.name,
    ))
}

fn field_failure(kind: FailureKind, field: &str) -> Failure {
    Failure::new(kind).with_context(DiagnosticContext {
        field: Some(field.to_owned()),
        ..DiagnosticContext::default()
    })
}

fn executable_token(value: &str) -> bool {
    !value.is_empty()
        && !value.chars().any(char::is_whitespace)
        && value.trim_matches(['.', ',', ';', ':', '(', ')', '[', ']']) == value
}

fn validate_executable_enum(field: &str, values: &[crate::EnumValue]) -> Result<(), Failure> {
    let mut lexical = Vec::new();
    for value in values {
        let canonical_is_token = executable_token(&value.value);
        if (!canonical_is_token && value.aliases.is_empty())
            || value.aliases.iter().any(|alias| !executable_token(alias))
        {
            return Err(field_failure(
                FailureKind::SchemaEnumDefinitionUnsupported,
                field,
            ));
        }
        for token in canonical_is_token
            .then_some(value.value.as_str())
            .into_iter()
            .chain(value.aliases.iter().map(String::as_str))
        {
            let normalized = token.to_ascii_lowercase();
            if lexical.contains(&normalized) {
                return Err(field_failure(
                    FailureKind::SchemaEnumDefinitionAmbiguous,
                    field,
                ));
            }
            lexical.push(normalized);
        }
    }
    Ok(())
}

/// Strict execution decoding; structural `TargetSchema::from_json` remains permissive.
/// Capability compilation is separate so CLI input-error precedence stays stable.
pub fn decode_execution_schema(input: &str) -> Result<TargetSchema, Failure> {
    let schema = TargetSchema::from_json(input).map_err(|error| Failure::from(&error))?;
    // Inspect the original JSON, not a serialization of the typed schema, which
    // has already discarded unknown properties. Structural errors keep priority.
    let value: serde_json::Value =
        serde_json::from_str(input).map_err(|_| Failure::new(FailureKind::SchemaParse))?;
    check_properties(
        &value,
        &["schema_version", "record_name", "fields", "options"],
    )?;
    check_properties(member(&value, "options", 3), &["allow_unknown_fields"])?;
    if let Some(fields) = member(&value, "fields", 2).as_array() {
        for field in fields {
            check_properties(
                field,
                &[
                    "name",
                    "field_type",
                    "required",
                    "multiple",
                    "aliases",
                    "constraints",
                ],
            )?;
            if let Some(constraints) = member(field, "constraints", 5).as_array() {
                for constraint in constraints {
                    check_properties(constraint, &["kind", "value"])?;
                }
            }
            // Structural decoding already enforces one known variant key and
            // also accepts unit variants as objects, e.g. {"email": null}.
            if let Some(enumeration) = member(field, "field_type", 1).get("enum") {
                check_properties(enumeration, &["values"])?;
                if let Some(values) = member(enumeration, "values", 0).as_array() {
                    for value in values {
                        check_properties(value, &["value", "aliases"])?;
                    }
                }
            }
        }
    }
    Ok(schema)
}

// Serde's existing structural decoder also accepts positional struct arrays.
// Traverse both representations, including objects nested inside arrays, rather
// than imposing a new wire shape or missing their unknown properties.
fn member<'a>(value: &'a serde_json::Value, key: &str, index: usize) -> &'a serde_json::Value {
    if value.is_array() {
        &value[index]
    } else {
        &value[key]
    }
}

fn check_properties(value: &serde_json::Value, allowed: &[&str]) -> Result<(), Failure> {
    if let Some(object) = value.as_object()
        && let Some(key) = object.keys().find(|key| !allowed.contains(&key.as_str()))
    {
        return Err(
            Failure::new(FailureKind::SchemaPropertyUnsupported).with_context(DiagnosticContext {
                value: Some(key.clone()),
                ..DiagnosticContext::default()
            }),
        );
    }
    Ok(())
}

/// JSON callers should use this entry point to retain strict property checks.
/// A typed schema cannot recover unknown JSON properties discarded upstream.
pub fn compile_schema_json(input: &str) -> Result<parser_core::ParsePlan, Failure> {
    compile_schema(&decode_execution_schema(input)?)
}

use super::*;

mod compilation;
mod text_names;

#[test]
fn all_validation_causes_have_exact_private_reports_and_opt_in_context() {
    use serde_json::json;
    let field = "private field 東京\n\u{1b}".to_owned();
    let value = "private value\t\"".to_owned();
    let alias = "private alias\\".to_owned();
    let cases = [
        (
            SchemaValidationError::EmptySchemaVersion,
            "empty_schema_version",
            "schema version must not be empty",
            None,
        ),
        (
            SchemaValidationError::UnsupportedSchemaVersion(value.clone()),
            "unsupported_schema_version",
            "unsupported schema version",
            Some(json!({"version": value})),
        ),
        (
            SchemaValidationError::EmptyFieldName,
            "empty_field_name",
            "field name must not be empty",
            None,
        ),
        (
            SchemaValidationError::DuplicateFieldName(field.clone()),
            "duplicate_field_name",
            "duplicate field name",
            Some(json!({"field": field})),
        ),
        (
            SchemaValidationError::DuplicateFieldLabel(value.clone()),
            "duplicate_field_label",
            "duplicate field label",
            Some(json!({"value": value})),
        ),
        (
            SchemaValidationError::EmptyAlias {
                field: field.clone(),
            },
            "empty_alias",
            "field has an empty alias",
            Some(json!({"field": field})),
        ),
        (
            SchemaValidationError::EmptyEnumValue {
                field: field.clone(),
            },
            "empty_enum_value",
            "enum field has an empty value",
            Some(json!({"field": field})),
        ),
        (
            SchemaValidationError::DuplicateEnumValue {
                field: field.clone(),
                value: value.clone(),
            },
            "duplicate_enum_value",
            "enum field repeats a value",
            Some(json!({"field": field, "value": value})),
        ),
        (
            SchemaValidationError::EmptyEnumAlias {
                field: field.clone(),
                value: value.clone(),
            },
            "empty_enum_alias",
            "enum value has an empty alias",
            Some(json!({"field": field, "value": value})),
        ),
        (
            SchemaValidationError::DuplicateEnumAlias {
                field: field.clone(),
                alias: alias.clone(),
            },
            "duplicate_enum_alias",
            "enum field repeats an alias",
            Some(json!({"field": field, "alias": alias})),
        ),
        (
            SchemaValidationError::InvalidIntegerRange {
                field: field.clone(),
            },
            "invalid_integer_range",
            "field has an invalid integer range",
            Some(json!({"field": field})),
        ),
        (
            SchemaValidationError::InvalidLengthRange {
                field: field.clone(),
            },
            "invalid_length_range",
            "field has an invalid length range",
            Some(json!({"field": field})),
        ),
    ];
    for (cause, reason, wording, context) in cases {
        let original = cause.clone();
        let message = format!("invalid schema: {wording}");
        let expected = json!({"error_contract_version": "0.1", "code": "schema_validation_error", "reason": reason});
        let safe = cause.report(DiagnosticsMode::Safe);
        assert_eq!(
            serde_json::to_value(&safe).unwrap(),
            json!({"error": expected, "message": message})
        );
        assert_eq!(cause.to_string(), message);
        assert_eq!(safe.to_string(), message);
        assert!(!serde_json::to_string(&safe).unwrap().contains("private"));

        let detailed = cause.report(DiagnosticsMode::Detailed);
        let mut expected_detailed = expected;
        let expected_message = if let Some(context) = context {
            expected_detailed["diagnostics"] = context;
            format!(
                "{message} [diagnostics: {}]",
                serde_json::to_string(&detailed.error.diagnostics).unwrap()
            )
        } else {
            message
        };
        assert_eq!(
            serde_json::to_value(&detailed).unwrap(),
            json!({"error": expected_detailed, "message": expected_message})
        );
        assert_eq!(detailed.to_string(), expected_message);
        assert!(!expected_message.contains(['\n', '\t', '\u{1b}']));
        assert_eq!(detailed, cause.report(DiagnosticsMode::Detailed));
        assert_eq!(cause, original);

        let nested = SchemaParseError::InvalidSchema(cause.clone());
        assert_eq!(
            nested
                .source()
                .unwrap()
                .downcast_ref::<SchemaValidationError>(),
            Some(&cause)
        );
        assert_eq!(nested.report(DiagnosticsMode::Safe), safe);
        let serialization = nested
            .serialization_failure()
            .report(DiagnosticsMode::Detailed);
        expected_detailed["code"] = json!("schema_serialization_error");
        expected_detailed.as_object_mut().unwrap().remove("reason");
        expected_detailed["cause"] = json!({"kind": "validation", "reason": reason});
        assert_eq!(
            serde_json::to_value(serialization.error).unwrap(),
            expected_detailed
        );
        assert_eq!(
            serde_json::to_value(nested.serialization_failure().report(DiagnosticsMode::Safe))
                .unwrap(),
            json!({
                "error": {"error_contract_version": "0.1", "code": "schema_serialization_error", "cause": {"kind": "validation", "reason": reason}},
                "message": "could not serialize schema"
            })
        );
    }
}

#[test]
fn opaque_schema_prose_stays_in_process_and_is_never_reconstructed_as_a_cause() {
    let cause = SchemaParseError::InvalidJson("upstream private record 東京\n\u{1b}".to_owned());
    assert!(cause.source().is_none());
    assert!(format!("{cause:?}").contains("upstream private"));
    assert_eq!(cause.to_string(), "invalid schema JSON");
    for mode in [DiagnosticsMode::Safe, DiagnosticsMode::Detailed] {
        assert_eq!(
            serde_json::to_value(cause.report(mode)).unwrap(),
            serde_json::json!({
                "error": {"error_contract_version": "0.1", "code": "schema_parse_error"},
                "message": "invalid schema JSON"
            })
        );
        assert_eq!(
            serde_json::to_value(cause.serialization_failure().report(mode)).unwrap(),
            serde_json::json!({
                "error": {"error_contract_version": "0.1", "code": "schema_serialization_error", "cause": {"kind": "json"}},
                "message": "could not serialize schema"
            })
        );
    }
    let invalid = TargetSchema {
        schema_version: "private-version".to_owned(),
        record_name: None,
        fields: vec![],
        options: SchemaOptions::default(),
    };
    let error = invalid.to_json().unwrap_err();
    assert_eq!(
        error,
        SchemaParseError::InvalidSchema(SchemaValidationError::UnsupportedSchemaVersion(
            "private-version".to_owned()
        ))
    );
    assert_eq!(
        error
            .source()
            .unwrap()
            .downcast_ref::<SchemaValidationError>(),
        Some(&SchemaValidationError::UnsupportedSchemaVersion(
            "private-version".to_owned()
        ))
    );
}

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
fn field_alias_collisions_are_rejected_case_insensitively() {
    let field = |name: &str, aliases: Vec<&str>| FieldDefinition {
        name: name.to_owned(),
        field_type: FieldType::Text,
        required: false,
        multiple: false,
        aliases: aliases.into_iter().map(str::to_owned).collect(),
        constraints: Vec::new(),
    };
    let schema = TargetSchema {
        schema_version: SCHEMA_VERSION.to_owned(),
        record_name: None,
        fields: vec![
            field("Email", vec!["contact"]),
            field("status", vec!["CONTACT"]),
        ],
        options: SchemaOptions::default(),
    };

    assert_eq!(
        schema.validate(),
        Err(SchemaValidationError::DuplicateFieldLabel(
            "CONTACT".to_owned()
        ))
    );
}

#[test]
fn enum_alias_collisions_are_rejected_case_insensitively() {
    let schema = TargetSchema {
        schema_version: SCHEMA_VERSION.to_owned(),
        record_name: None,
        fields: vec![FieldDefinition {
            name: "status".to_owned(),
            field_type: FieldType::Enum {
                values: vec![
                    EnumValue {
                        value: "active".to_owned(),
                        aliases: vec!["enabled".to_owned()],
                    },
                    EnumValue {
                        value: "inactive".to_owned(),
                        aliases: vec!["ENABLED".to_owned()],
                    },
                ],
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
        Err(SchemaValidationError::DuplicateEnumAlias {
            field: "status".to_owned(),
            alias: "ENABLED".to_owned(),
        })
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

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

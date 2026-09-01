use super::*;
use parser_core::{FailureKind, ResourceLimitKind};
use serde_json::{Value, json};

fn field(name: &str, field_type: Value, aliases: &[&str]) -> Value {
    json!({
        "name": name,
        "field_type": field_type,
        "required": false,
        "multiple": false,
        "aliases": aliases,
        "constraints": []
    })
}

fn profile(fields: Vec<Value>) -> String {
    json!({
        "schema_version": "0.1",
        "record_name": "limited",
        "fields": fields,
        "options": {"allow_unknown_fields": true}
    })
    .to_string()
}

fn limits() -> SchemaLimits {
    SchemaLimits {
        max_bytes: usize::MAX,
        max_fields: usize::MAX,
        max_aliases: usize::MAX,
        max_nesting: usize::MAX,
    }
}

fn assert_limit(failure: Failure, resource: ResourceLimitKind, limit: u64, actual: u64) {
    assert_eq!(
        failure.kind,
        FailureKind::ResourceLimit {
            resource,
            limit,
            actual,
        }
    );
}

#[test]
fn schema_bytes_and_fields_accept_exact_limits_and_reject_one_over() {
    let input = profile(vec![
        field("email", json!("email"), &[]),
        field("enabled", json!("boolean"), &[]),
    ]);
    let exact = SchemaLimits {
        max_bytes: input.len(),
        max_fields: 2,
        ..limits()
    };
    assert!(decode_execution_schema_with_limits(&input, exact).is_ok());
    assert!(decode_schema_with_limits(&input, exact).is_ok());

    assert_limit(
        decode_execution_schema_with_limits(
            &input,
            SchemaLimits {
                max_bytes: input.len() - 1,
                ..exact
            },
        )
        .unwrap_err(),
        ResourceLimitKind::SchemaBytes,
        (input.len() - 1) as u64,
        input.len() as u64,
    );
    assert_limit(
        decode_schema_with_limits(
            &input,
            SchemaLimits {
                max_fields: 1,
                ..exact
            },
        )
        .unwrap_err(),
        ResourceLimitKind::SchemaFields,
        1,
        2,
    );
}

#[test]
fn field_and_enum_aliases_share_one_limit_for_object_and_sequence_schemas() {
    let object = profile(vec![field(
        "state",
        json!({"enum":{"values":[{"value":"active","aliases":["go"]}]}}),
        &["status"],
    )]);
    let sequence = json!([
        "0.1",
        null,
        [[
            "state",
            {"enum":[[["active",["go"]]]]},
            false,
            false,
            ["status"],
            []
        ]],
        [true]
    ])
    .to_string();
    for input in [object, sequence] {
        let exact = SchemaLimits {
            max_aliases: 2,
            ..limits()
        };
        assert!(decode_execution_schema_with_limits(&input, exact).is_ok());
        assert_limit(
            decode_execution_schema_with_limits(
                &input,
                SchemaLimits {
                    max_aliases: 1,
                    ..exact
                },
            )
            .unwrap_err(),
            ResourceLimitKind::SchemaAliases,
            1,
            2,
        );
    }
}

#[test]
fn schema_nesting_accepts_exact_container_depth_and_rejects_one_over() {
    let input = profile(vec![field(
        "state",
        json!({"enum":{"values":[{"value":"active","aliases":["go"]}]}}),
        &[],
    )]);
    let mut value: Value = serde_json::from_str(&input).unwrap();
    value["record_name"] = json!(r#"literal [[{{\" boundaries"#);
    let input = value.to_string();
    let exact = SchemaLimits {
        max_nesting: 8,
        ..limits()
    };
    assert!(decode_execution_schema_with_limits(&input, exact).is_ok());
    assert!(decode_schema_with_limits(&input, exact).is_ok());
    assert_limit(
        decode_execution_schema_with_limits(
            &input,
            SchemaLimits {
                max_nesting: 7,
                ..exact
            },
        )
        .unwrap_err(),
        ResourceLimitKind::SchemaNesting,
        7,
        8,
    );
}

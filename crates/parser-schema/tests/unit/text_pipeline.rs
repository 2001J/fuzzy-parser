use super::*;
use parser_core::{ParseContent, TextCompositionSegment};
use serde_json::{Value, json};

fn pipeline(strategy: &str, markers: &[&str]) -> Value {
    json!({
        "normalization": {
            "normalize_line_endings": true,
            "trim_whitespace": true,
            "collapse_whitespace": true,
            "normalize_punctuation": true,
            "mark_noise": true
        },
        "strategy": strategy,
        "repeated_identifier_markers": markers
    })
}

fn profile_with_pipeline(strategy: &str, markers: &[&str]) -> Value {
    let mut profile =
        super::compilation::profile(vec![super::compilation::field("email", json!("email"))]);
    profile["options"]["text_pipeline"] = pipeline(strategy, markers);
    profile
}

#[test]
fn old_schema_and_response_shapes_omit_additive_pipeline_members() {
    let profile = super::compilation::profile(Vec::new());
    let typed = TargetSchema::from_json(&profile.to_string()).unwrap();
    assert!(typed.options.text_pipeline.is_none());
    assert_eq!(serde_json::to_value(&typed).unwrap(), profile);
    let response = parser_core::parse_document_with_plan(
        &super::compilation::document("unchanged"),
        &compile_schema(&typed).unwrap(),
    );
    let encoded = serde_json::to_value(&response).unwrap();
    assert!(
        encoded["content"]["records"][0]
            .get("composition")
            .is_none()
    );
    let decoded: parser_core::ParseResponse = serde_json::from_value(encoded).unwrap();
    assert_eq!(decoded, response);
}

#[test]
fn strict_execution_checks_unknown_text_pipeline_properties_at_both_levels() {
    let base = profile_with_pipeline("one_block_per_record", &[]);
    for path in ["pipeline", "normalization"] {
        let mut profile = base.clone();
        if path == "pipeline" {
            profile["options"]["text_pipeline"]["private"] = json!(true);
        } else {
            profile["options"]["text_pipeline"]["normalization"]["private"] = json!(true);
        }
        assert_eq!(
            decode_execution_schema(&profile.to_string())
                .unwrap_err()
                .kind,
            parser_core::FailureKind::SchemaPropertyUnsupported
        );
    }
    assert!(decode_execution_schema(&base.to_string()).is_ok());
}

#[test]
fn invalid_repeated_marker_combinations_use_existing_safe_option_failure() {
    for (strategy, markers) in [
        ("split_repeated_identifiers", vec![]),
        ("split_repeated_identifiers", vec![""]),
        ("split_repeated_identifiers", vec!["entry"]),
        ("split_repeated_identifiers", vec!["entry:\n"]),
        ("split_repeated_identifiers", vec!["entry:", "ENTRY:"]),
        ("one_block_per_record", vec!["entry:"]),
        ("join_indented_continuations", vec!["entry:"]),
    ] {
        let failure = compile_schema_json(&profile_with_pipeline(strategy, &markers).to_string())
            .unwrap_err();
        assert_eq!(
            failure.kind,
            parser_core::FailureKind::SchemaOptionUnsupported
        );
        let safe =
            serde_json::to_value(failure.report(parser_core::DiagnosticsMode::Safe)).unwrap();
        assert_eq!(safe["error"]["code"], "schema_option_unsupported");
        assert!(safe["error"].get("diagnostics").is_none());
    }
}

#[test]
fn compiled_and_typed_plans_share_repeated_split_execution() {
    let profile = profile_with_pipeline("split_repeated_identifiers", &["entry id:"]);
    let document =
        super::compilation::document("entry id: ada@example.test entry id: grace@example.test");
    let json_plan = compile_schema_json(&profile.to_string()).unwrap();
    let typed = TargetSchema::from_json(&profile.to_string()).unwrap();
    let typed_plan = compile_schema(&typed).unwrap();
    let response = parser_core::parse_document_with_plan(&document, &json_plan);
    assert_eq!(
        response,
        parser_core::parse_document_with_plan(&document, &typed_plan)
    );
    let ParseContent::Text { records } = &response.content else {
        panic!("expected text mode");
    };
    assert_eq!(records.len(), 2);
    for record in records {
        let composition = record.composition.as_ref().unwrap();
        assert_eq!(
            composition.applied_options.repeated_identifier_markers,
            ["entry id:"]
        );
        let TextCompositionSegment::Source {
            source_reference, ..
        } = &composition.segments[0]
        else {
            panic!("split records have one source segment");
        };
        assert_eq!(source_reference.block_index, 0);
        let candidate = &record.parse.assignment.fields[0].candidates[0];
        assert_eq!(
            candidate
                .source_reference
                .as_ref()
                .unwrap()
                .resolve(&document)
                .as_deref(),
            Some(candidate.raw_value.as_str())
        );
    }
}

#[test]
fn omitted_required_pipeline_members_remain_schema_parse_errors() {
    for member in ["normalization", "strategy"] {
        let mut profile = profile_with_pipeline("one_block_per_record", &[]);
        profile["options"]["text_pipeline"]
            .as_object_mut()
            .unwrap()
            .remove(member);
        assert_eq!(
            Failure::from(&TargetSchema::from_json(&profile.to_string()).unwrap_err()).kind,
            parser_core::FailureKind::SchemaParse
        );
    }
}

use super::*;
use parser_core::{
    ParseContent, RawBlock, RawDocument, RawValue, SourceLocation, SourceMetadata, SourceType,
    parse_document_with_plan,
};
use serde_json::{Value, json};

pub(super) fn field(name: &str, kind: Value) -> Value {
    json!({"name":name,"field_type":kind,"required":true,"multiple":false,"aliases":[],"constraints":[]})
}

fn enumeration(name: &str, canonical: &str, aliases: &[&str]) -> Value {
    field(
        name,
        json!({"enum":{"values":[{"value":canonical,"aliases":aliases}]}}),
    )
}

pub(super) fn profile(fields: Vec<Value>) -> Value {
    json!({"schema_version":"0.1","record_name":"synthetic","fields":fields,"options":{"allow_unknown_fields":true}})
}

pub(super) fn document(text: &str) -> RawDocument {
    RawDocument::new(
        "synthetic",
        SourceMetadata {
            source_type: SourceType::Text,
            file_name: None,
            mime_type: None,
            size_bytes: Some(text.len() as u64),
            delimiter: None,
        },
        vec![RawBlock {
            id: "record".into(),
            value: RawValue::text(text),
            location: SourceLocation::default(),
        }],
    )
}

pub(super) fn parse(profile: &Value, document: &RawDocument) -> parser_core::ParseResponse {
    let plan = compile_schema_json(&profile.to_string()).unwrap();
    let response = parse_document_with_plan(document, &plan);
    let typed = TargetSchema::from_json(&profile.to_string()).unwrap();
    assert_eq!(
        response,
        parse_document_with_plan(document, &compile_schema(&typed).unwrap())
    );
    assert_eq!(response, parse_document_with_plan(document, &plan));
    assert_eq!(
        response.source_evidence.as_ref().unwrap().document,
        *document
    );
    assert_sources(&response);
    response
}

pub(super) fn parses(response: &parser_core::ParseResponse) -> Vec<&parser_core::TextParseResult> {
    match &response.content {
        ParseContent::Text { records } => records.iter().map(|r| &r.parse).collect(),
        ParseContent::Table { sheets } => sheets
            .iter()
            .flat_map(|s| s.records.iter().map(|r| &r.parse))
            .collect(),
    }
}

fn assert_sources(response: &parser_core::ParseResponse) {
    let evidence = response.source_evidence.as_ref().unwrap();
    let mut covered: Vec<_> = evidence
        .document
        .blocks
        .iter()
        .map(|b| vec![false; b.value.to_text().len()])
        .collect();
    for parse in parses(response) {
        for candidate in parse
            .candidates
            .iter()
            .chain(parse.assignment.fields.iter().flat_map(|f| &f.candidates))
            .chain(&parse.assignment.unassigned_candidates)
        {
            let reference = candidate.source_reference.as_ref().unwrap();
            assert_eq!(
                reference.resolve(&evidence.document).as_deref(),
                Some(candidate.raw_value.as_str())
            );
            assert!(
                parse
                    .candidates
                    .iter()
                    .any(|c| c.source_reference == candidate.source_reference
                        && c.normalized_value == candidate.normalized_value
                        && c.candidate_type == candidate.candidate_type)
            );
            covered[reference.block_index][reference.span.byte_start..reference.span.byte_end]
                .fill(true);
        }
        let enum_refs: Vec<_> = parse
            .assignment
            .fields
            .iter()
            .flat_map(|f| &f.candidates)
            .filter(|c| c.candidate_type == parser_core::CandidateType::Enum)
            .map(|c| c.source_reference.as_ref().unwrap())
            .collect();
        for (i, reference) in enum_refs.iter().enumerate() {
            assert!(
                !enum_refs[..i].contains(reference),
                "an enum occurrence must not be assigned twice"
            );
        }
    }
    for coverage in &evidence.blocks {
        if coverage.role != parser_core::SourceBlockRole::Parsed {
            continue;
        }
        for span in &coverage.unused_spans {
            for byte in &mut covered[coverage.block_index][span.byte_start..span.byte_end] {
                assert!(!*byte);
                *byte = true;
            }
        }
        assert!(covered[coverage.block_index].iter().all(|b| *b));
    }
}

fn codes(parse: &parser_core::TextParseResult) -> Vec<&str> {
    parse
        .assignment
        .warnings
        .iter()
        .map(|w| w.code.as_str())
        .collect()
}

#[test]
fn compiled_disjoint_enums_keep_ownership_in_both_field_orders() {
    for fields in [
        vec![
            enumeration("color", "red", &[]),
            enumeration("state", "enabled", &[]),
        ],
        vec![
            enumeration("state", "enabled", &[]),
            enumeration("color", "red", &[]),
        ],
    ] {
        for text in ["red enabled", "enabled red"] {
            let response = parse(&profile(fields.clone()), &document(text));
            let parsed = parses(&response)[0];
            assert!(codes(parsed).is_empty());
            for assigned in &parsed.assignment.fields {
                assert_eq!(
                    assigned.candidates[0].normalized_value,
                    Some(json!(if assigned.name == "color" {
                        "red"
                    } else {
                        "enabled"
                    }))
                );
            }
        }
    }
}

#[test]
fn shared_canonical_does_not_share_another_fields_alias() {
    let response = parse(
        &profile(vec![
            enumeration("first", "active", &["go"]),
            enumeration("second", "active", &["run"]),
        ]),
        &document("run"),
    );
    let parsed = parses(&response)[0];
    assert_eq!(parsed.assignment.fields.len(), 1);
    assert_eq!(parsed.assignment.fields[0].name, "second");
    assert_eq!(codes(parsed), ["required_field_missing"]);
}

#[test]
fn overlapping_aliases_use_unique_label_context() {
    let response = parse(
        &profile(vec![
            enumeration("first", "active", &["go"]),
            enumeration("second", "approved", &["go"]),
        ]),
        &document("第二 second: go"),
    );
    let parsed = parses(&response)[0];
    assert_eq!(parsed.assignment.fields.len(), 1);
    assert_eq!(parsed.assignment.fields[0].name, "second");
    assert_eq!(
        parsed.assignment.fields[0].candidates[0].normalized_value,
        Some(json!("approved"))
    );
    assert!(!codes(parsed).contains(&"enum_field_ambiguous"));
    assert_eq!(parsed.candidates.len(), 2);
    assert_eq!(parsed.assignment.unassigned_candidates.len(), 1);
}

#[test]
fn overlapping_aliases_follow_opposite_table_headers() {
    let mut input = document("");
    input.blocks = ["second", "first", "  go  ", "go"]
        .iter()
        .enumerate()
        .map(|(i, text)| RawBlock {
            id: format!("cell-{i}"),
            value: RawValue::text(*text),
            location: SourceLocation {
                row: Some(i / 2 + 1),
                column: Some(i % 2 + 1),
                ..SourceLocation::default()
            },
        })
        .collect();
    let response = parse(
        &profile(vec![
            enumeration("first", "active", &["go"]),
            enumeration("second", "approved", &["go"]),
        ]),
        &input,
    );
    let parsed = parses(&response)[0];
    assert!(codes(parsed).is_empty());
    for assigned in &parsed.assignment.fields {
        let c = &assigned.candidates[0];
        let first = assigned.name == "first";
        assert_eq!(
            c.normalized_value,
            Some(json!(if first { "active" } else { "approved" }))
        );
        assert_eq!(
            c.source_reference.as_ref().unwrap().block_index,
            if first { 3 } else { 2 }
        );
        assert_eq!(
            c.source_reference.as_ref().unwrap().span,
            parser_core::TextSpan {
                byte_start: if first { 0 } else { 2 },
                byte_end: if first { 2 } else { 4 }
            }
        );
    }
}

#[test]
fn unlabeled_overlap_preserves_hypotheses_without_double_assignment() {
    for second in ["active", "approved"] {
        let response = parse(
            &profile(vec![
                enumeration("first", "active", &["go"]),
                enumeration("second", second, &["go"]),
            ]),
            &document("go go, note"),
        );
        let parsed = parses(&response)[0];
        assert!(parsed.assignment.fields.is_empty());
        assert_eq!(
            codes(parsed)
                .iter()
                .filter(|code| **code == "enum_field_ambiguous")
                .count(),
            2
        );
        assert_eq!(
            codes(parsed)
                .iter()
                .filter(|code| **code == "required_field_missing")
                .count(),
            2
        );
        assert_eq!(parsed.assignment.unassigned_candidates, parsed.candidates);
        assert_eq!(
            parsed.candidates.len(),
            if second == "active" { 2 } else { 4 }
        );
    }
}

#[test]
fn constraints_do_not_invent_enum_ownership() {
    let mut first = enumeration("first", "active", &["go"]);
    first["constraints"] = json!([{"kind":"maximum_length","value":1}]);
    let response = parse(
        &profile(vec![first, enumeration("second", "approved", &["go"])]),
        &document("go"),
    );
    assert!(parses(&response)[0].assignment.fields.is_empty());
    assert!(codes(parses(&response)[0]).contains(&"enum_field_ambiguous"));
}

#[test]
fn within_field_lexical_collisions_are_rejected_without_changing_structural_validation() {
    for values in [
        json!([{"value":"Active","aliases":[]},{"value":"active","aliases":[]}]),
        json!([{"value":"active","aliases":["enabled"]},{"value":"enabled","aliases":[]}]),
    ] {
        let schema = profile(vec![field("state", json!({"enum":{"values":values}}))]);
        let structural = TargetSchema::from_json(&schema.to_string()).unwrap();
        assert_eq!(
            compile_schema(&structural).unwrap_err().kind,
            FailureKind::SchemaEnumDefinitionAmbiguous
        );
    }
}

#[test]
fn empty_enum_and_multiword_canonical_alias_are_explicit() {
    let response = parse(
        &profile(vec![field("empty", json!({"enum":{"values":[]}}))]),
        &document("unknown"),
    );
    assert!(parses(&response)[0].candidates.is_empty());
    assert_eq!(codes(parses(&response)[0]), ["required_field_missing"]);
    let response = parse(
        &profile(vec![enumeration("state", "in stock", &["ready"])]),
        &document("READY"),
    );
    assert_eq!(
        parses(&response)[0].assignment.fields[0].candidates[0].normalized_value,
        Some(json!("in stock"))
    );
}

#[test]
fn undetectable_enum_definition_shapes_are_rejected() {
    for (canonical, aliases) in [
        ("in stock", vec![]),
        ("active", vec!["in stock"]),
        ("active", vec!["ready."]),
        (".active", vec![]),
    ] {
        let schema = profile(vec![enumeration("state", canonical, &aliases)]);
        assert_eq!(
            compile_schema_json(&schema.to_string()).unwrap_err().kind,
            FailureKind::SchemaEnumDefinitionUnsupported
        );
    }
}

#[test]
fn programmatic_schemas_are_structurally_validated_by_compilation() {
    let mut schema = TargetSchema::from_json(&profile(vec![]).to_string()).unwrap();
    schema.schema_version = "invalid".into();
    assert_eq!(
        compile_schema(&schema).unwrap_err().kind,
        FailureKind::SchemaValidation {
            reason: SchemaValidationReason::UnsupportedSchemaVersion
        }
    );
    let valid =
        TargetSchema::from_json(&profile(vec![field("count", json!("integer"))]).to_string())
            .unwrap();
    let mut duplicate = valid.clone();
    duplicate.fields.push(duplicate.fields[0].clone());
    assert_eq!(
        compile_schema(&duplicate).unwrap_err().kind,
        FailureKind::SchemaValidation {
            reason: SchemaValidationReason::DuplicateFieldName
        }
    );
    let mut range = valid;
    range.fields[0].constraints = vec![
        FieldConstraint::MinimumInteger(2),
        FieldConstraint::MaximumInteger(1),
    ];
    assert_eq!(
        compile_schema(&range).unwrap_err().kind,
        FailureKind::SchemaValidation {
            reason: SchemaValidationReason::InvalidIntegerRange
        }
    );
}

#[test]
fn constraint_applicability_matrix_and_unsupported_types_are_explicit() {
    for kind in [
        json!("integer"),
        json!("decimal"),
        json!("currency"),
        json!("boolean"),
        json!("email"),
        json!("phone_number"),
        json!("date"),
        json!("text"),
        json!("person_name"),
        json!({"enum":{"values":[]}}),
    ] {
        for constraint in [
            "minimum_integer",
            "maximum_integer",
            "minimum_length",
            "maximum_length",
        ] {
            let mut f = field("value", kind.clone());
            f["constraints"] = json!([{"kind":constraint,"value":1}]);
            let result = compile_schema_json(&profile(vec![f]).to_string());
            let applicable = if constraint.ends_with("integer") {
                kind == "integer"
            } else {
                kind == "email"
                    || kind == "phone_number"
                    || kind == "date"
                    || kind == "text"
                    || kind == "person_name"
                    || kind.is_object()
            };
            if applicable {
                assert!(result.is_ok());
            } else {
                assert_eq!(
                    result.unwrap_err().kind,
                    FailureKind::SchemaConstraintUnsupported
                );
            }
        }
    }
    assert_eq!(
        compile_schema_json(&profile(vec![field("value", json!("datetime"))]).to_string())
            .unwrap_err()
            .kind,
        FailureKind::SchemaFieldTypeUnsupported {
            field_type: parser_core::UnsupportedFieldType::Datetime
        }
    );
}

#[test]
fn inclusive_repeated_ranges_and_required_multiple_preserve_all_evidence() {
    let mut f = field("count", json!("integer"));
    f["multiple"] = json!(true);
    f["constraints"] = json!([{"kind":"minimum_integer","value":1},{"kind":"minimum_integer","value":2},{"kind":"maximum_integer","value":3}]);
    let response = parse(&profile(vec![f.clone()]), &document("1 2 3 4"));
    let parsed = parses(&response)[0];
    assert_eq!(
        parsed.assignment.fields[0]
            .candidates
            .iter()
            .map(|c| c.raw_value.as_str())
            .collect::<Vec<_>>(),
        ["2", "3"]
    );
    assert_eq!(
        parsed
            .assignment
            .unassigned_candidates
            .iter()
            .map(|c| c.raw_value.as_str())
            .collect::<Vec<_>>(),
        ["1", "4"]
    );
    f["constraints"]
        .as_array_mut()
        .unwrap()
        .push(json!({"kind":"minimum_integer","value":5}));
    let response = parse(&profile(vec![f.clone()]), &document("2 3"));
    assert_eq!(codes(parses(&response)[0]), ["required_field_missing"]);
    f["required"] = json!(false);
    let response = parse(&profile(vec![f]), &document("2 3"));
    assert!(codes(parses(&response)[0]).is_empty());
}

#[test]
fn normalized_lengths_measure_digits_iso_dates_and_canonical_unicode_scalars() {
    for (mut f, input, length) in [
        (field("phone", json!("phone_number")), "+1-202-555-0100", 11),
        (field("date", json!("date")), "2026/08/28", 10),
        (field("email", json!("email")), "ADA@example.test", 16),
        (enumeration("state", "日 本", &["ready"]), "ready", 3),
    ] {
        f["constraints"] = json!([{"kind":"minimum_length","value":length},{"kind":"maximum_length","value":length}]);
        let response = parse(&profile(vec![f.clone()]), &document(input));
        assert_eq!(parses(&response)[0].assignment.fields.len(), 1);
        f["constraints"][1]["value"] = json!(length - 1);
        // Keep structural min/max valid while exercising a failing runtime bound.
        f["constraints"][0]["value"] = json!(0);
        let response = parse(&profile(vec![f]), &document(input));
        assert!(parses(&response)[0].assignment.fields.is_empty());
    }
}

#[test]
fn strict_execution_checks_unknown_properties_at_each_modeled_object() {
    let base = profile(vec![enumeration("state", "active", &["go"])]);
    for pointer in [
        "",
        "/options",
        "/fields/0",
        "/fields/0/field_type/enum",
        "/fields/0/field_type/enum/values/0",
    ] {
        let mut raw = base.clone();
        raw.pointer_mut(pointer).unwrap()["private-unknown"] = json!(true);
        let input = raw.to_string();
        assert!(TargetSchema::from_json(&input).is_ok());
        assert_eq!(
            decode_execution_schema(&input).unwrap_err().kind,
            FailureKind::SchemaPropertyUnsupported
        );
    }
    let mut raw = base;
    raw["fields"][0]["constraints"] =
        json!([{"kind":"minimum_length","value":1,"private-unknown":true}]);
    assert!(TargetSchema::from_json(&raw.to_string()).is_ok());
    assert_eq!(
        decode_execution_schema(&raw.to_string()).unwrap_err().kind,
        FailureKind::SchemaPropertyUnsupported
    );
}

#[test]
fn omitted_required_members_and_structural_errors_keep_existing_codes() {
    for member in ["schema_version", "fields", "options"] {
        let mut raw = profile(vec![]);
        raw.as_object_mut().unwrap().remove(member);
        assert_eq!(
            decode_execution_schema(&raw.to_string()).unwrap_err().kind,
            FailureKind::SchemaParse
        );
    }
    for member in [
        "name",
        "field_type",
        "required",
        "multiple",
        "aliases",
        "constraints",
    ] {
        let mut raw = profile(vec![field("value", json!("integer"))]);
        raw["fields"][0].as_object_mut().unwrap().remove(member);
        assert_eq!(
            decode_execution_schema(&raw.to_string()).unwrap_err().kind,
            FailureKind::SchemaParse
        );
    }
    let mut raw = profile(vec![]);
    raw["options"] = json!({});
    assert_eq!(
        decode_execution_schema(&raw.to_string()).unwrap_err().kind,
        FailureKind::SchemaParse
    );
    for (pointer, member) in [
        ("/fields/0/field_type/enum", "values"),
        ("/fields/0/field_type/enum/values/0", "value"),
        ("/fields/0/field_type/enum/values/0", "aliases"),
        ("/fields/0/constraints/0", "kind"),
        ("/fields/0/constraints/0", "value"),
    ] {
        let mut raw = profile(vec![enumeration("state", "active", &["go"])]);
        raw["fields"][0]["constraints"] = json!([{"kind":"minimum_length","value":1}]);
        raw.pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove(member);
        assert_eq!(
            decode_execution_schema(&raw.to_string()).unwrap_err().kind,
            FailureKind::SchemaParse
        );
    }
}

#[test]
fn permissive_option_preserves_unused_input_and_false_is_unsupported() {
    let mut raw = profile(vec![]);
    let response = parse(&raw, &document("unrecognized 42"));
    assert_eq!(
        parses(&response)[0].assignment.unassigned_candidates.len(),
        1
    );
    assert_eq!(
        response.source_evidence.as_ref().unwrap().blocks[0].unused_spans,
        vec![parser_core::TextSpan {
            byte_start: 0,
            byte_end: 13
        }]
    );
    raw["options"]["allow_unknown_fields"] = json!(false);
    assert!(TargetSchema::from_json(&raw.to_string()).is_ok());
    assert_eq!(
        compile_schema_json(&raw.to_string()).unwrap_err().kind,
        FailureKind::SchemaOptionUnsupported
    );
}

#[test]
fn capability_failures_have_exact_safe_reports_and_opt_in_context() {
    for (kind, code, message) in [
        (
            FailureKind::SchemaOptionUnsupported,
            "schema_option_unsupported",
            "schema option is not supported for execution",
        ),
        (
            FailureKind::SchemaConstraintUnsupported,
            "schema_constraint_unsupported",
            "schema constraint is not supported for this field type",
        ),
        (
            FailureKind::SchemaPropertyUnsupported,
            "schema_property_unsupported",
            "schema property is not supported for execution",
        ),
        (
            FailureKind::SchemaEnumDefinitionAmbiguous,
            "schema_enum_definition_ambiguous",
            "enum field has ambiguous lexical definitions",
        ),
        (
            FailureKind::SchemaEnumDefinitionUnsupported,
            "schema_enum_definition_unsupported",
            "enum definition cannot be detected by the current parser",
        ),
    ] {
        let error = Failure::new(kind).with_context(DiagnosticContext {
            field: Some("private 東京\n\u{1b}".into()),
            ..DiagnosticContext::default()
        });
        let expected =
            json!({"error":{"error_contract_version":"0.1","code":code},"message":message});
        assert_eq!(
            serde_json::to_value(error.report(DiagnosticsMode::Safe)).unwrap(),
            expected
        );
        assert_eq!(error.to_string(), message);
        assert_eq!(serde_json::to_value(&error).unwrap(), expected["error"]);
        for mode in [DiagnosticsMode::Safe, DiagnosticsMode::Detailed] {
            let report = error.report(mode);
            let encoded = serde_json::to_string(&report).unwrap();
            assert_eq!(
                serde_json::from_str::<ErrorReport>(&encoded).unwrap(),
                report
            );
            assert_eq!(report.message(), report.to_string());
            assert!(!encoded.contains('\u{1b}'));
        }
    }
}

#[test]
fn repeated_enum_occurrences_respect_single_multiple_and_tied_labels() {
    let mut f = enumeration("state", "active", &["go"]);
    for multiple in [false, true] {
        f["multiple"] = json!(multiple);
        let response = parse(&profile(vec![f.clone()]), &document("東京 (go), go."));
        let parsed = parses(&response)[0];
        let expected = if multiple { 2 } else { 1 };
        assert_eq!(parsed.assignment.fields[0].candidates.len(), expected);
        assert_eq!(parsed.assignment.unassigned_candidates.len(), 2 - expected);
        assert_eq!(
            codes(parsed),
            if multiple {
                vec![]
            } else {
                vec!["multiple_candidates_ambiguous"]
            }
        );
        assert_eq!(
            parsed.assignment.fields[0].candidates[0].source_span,
            parser_core::TextSpan {
                byte_start: 8,
                byte_end: 10
            }
        );
    }
    let response = parse(
        &profile(vec![
            enumeration("first", "active", &["go"]),
            enumeration("second", "approved", &["go"]),
        ]),
        &document("first: second: go"),
    );
    assert!(parses(&response)[0].assignment.fields.is_empty());
    assert_eq!(
        codes(parses(&response)[0]),
        [
            "enum_field_ambiguous",
            "required_field_missing",
            "required_field_missing"
        ]
    );
}

#[test]
fn repeated_normalized_length_bounds_are_all_enforced() {
    let mut f = enumeration("state", "日 本", &["ready"]);
    f["constraints"] = json!([{"kind":"minimum_length","value":1},{"kind":"maximum_length","value":5},{"kind":"minimum_length","value":3},{"kind":"maximum_length","value":3}]);
    let response = parse(&profile(vec![f.clone()]), &document("READY"));
    assert_eq!(parses(&response)[0].assignment.fields.len(), 1);
    f["constraints"]
        .as_array_mut()
        .unwrap()
        .push(json!({"kind":"maximum_length","value":2}));
    let response = parse(&profile(vec![f]), &document("READY"));
    assert!(parses(&response)[0].assignment.fields.is_empty());
    assert_eq!(codes(parses(&response)[0]), ["required_field_missing"]);
}

#[test]
fn empty_plan_keeps_every_builtin_detector_and_profile_names_are_opaque() {
    let input = document("Ada,ada@example.test 42 12.5 +1-202-555-0100 true 2026/08/28 $19.95");
    let mut schema = profile(vec![]);
    let original = parse(&schema, &input);
    let candidates = &parses(&original)[0].candidates;
    for kind in [
        parser_core::CandidateType::Email,
        parser_core::CandidateType::Integer,
        parser_core::CandidateType::Decimal,
        parser_core::CandidateType::PhoneNumber,
        parser_core::CandidateType::Boolean,
        parser_core::CandidateType::Date,
        parser_core::CandidateType::Currency,
    ] {
        assert!(
            candidates
                .iter()
                .any(|candidate| candidate.candidate_type == kind)
        );
    }
    assert_eq!(
        *candidates,
        parses(&original)[0].assignment.unassigned_candidates
    );
    for name in ["attendance_draft", "stock_check", "unrelated 東京"] {
        schema["record_name"] = json!(name);
        let mut response = parse(&schema, &input);
        assert_eq!(response.record_name.as_deref(), Some(name));
        response.record_name = original.record_name.clone();
        assert_eq!(response, original);
    }
}

#[test]
fn execution_preserves_structural_sequence_and_unit_variant_encodings() {
    let ordinary = profile(vec![field("email", json!("email"))]);
    let expected = parse(&ordinary, &document("ada@example.test"));
    let mut wrapper = ordinary.clone();
    wrapper["fields"][0]["field_type"] = json!({"email":null});
    let sequence = json!(["0.1","synthetic",[["email",{"email":null},true,false,[],[]]],[true]]);
    for encoding in [wrapper, sequence] {
        assert_eq!(parse(&encoding, &document("ada@example.test")), expected);
    }
    let enum_sequence = json!(["0.1","synthetic",[["state",{"enum":[[["active",["go"]]]]},true,false,[],[]]],[true]]);
    let expected = parse(
        &profile(vec![enumeration("state", "active", &["go"])]),
        &document("go"),
    );
    assert_eq!(parse(&enum_sequence, &document("go")), expected);
}

#[test]
fn unknown_properties_are_checked_inside_structural_sequences() {
    let mut raw = json!(["0.1",null,[],{"allow_unknown_fields":true,"private":true}]);
    assert!(TargetSchema::from_json(&raw.to_string()).is_ok());
    assert_eq!(
        decode_execution_schema(&raw.to_string()).unwrap_err().kind,
        FailureKind::SchemaPropertyUnsupported
    );
    raw = json!(["0.1",null,[["state",{"enum":[[{"value":"active","aliases":[],"private":true}]]},false,false,[],[]]],[true]]);
    assert!(TargetSchema::from_json(&raw.to_string()).is_ok());
    assert_eq!(
        decode_execution_schema(&raw.to_string()).unwrap_err().kind,
        FailureKind::SchemaPropertyUnsupported
    );
}

#[test]
fn existing_type_capability_errors_precede_new_compilation_failures() {
    let mut invalid = field("email", json!("email"));
    invalid["constraints"] = json!([{"kind":"minimum_integer","value":1}]);
    let mut raw = profile(vec![
        invalid,
        field("text", json!("text")),
        field("time", json!("datetime")),
    ]);
    for allow_unknown in [true, false] {
        raw["options"]["allow_unknown_fields"] = json!(allow_unknown);
        assert_eq!(
            compile_schema_json(&raw.to_string()).unwrap_err().kind,
            FailureKind::SchemaFieldTypeUnsupported {
                field_type: parser_core::UnsupportedFieldType::Datetime
            }
        );
    }
}

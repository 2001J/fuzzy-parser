use super::compilation::{document, field, parse, parses, profile};
use super::*;
use parser_core::{
    CandidateType, FieldCandidate, ParseResponse, RawBlock, RawDocument, RawValue, SourceLocation,
};
use serde_json::{Value, json};

fn checked(schema: &Value, doc: &RawDocument) -> ParseResponse {
    let response = parse(schema, doc);
    for parsed in parses(&response) {
        let assigned: Vec<_> = parsed
            .assignment
            .fields
            .iter()
            .flat_map(|f| &f.candidates)
            .collect();
        for (i, a) in assigned.iter().enumerate() {
            if !matches!(
                a.candidate_type,
                CandidateType::Text | CandidateType::PersonName
            ) {
                continue;
            }
            let a = a.source_reference.as_ref().unwrap();
            for (j, b) in assigned.iter().enumerate() {
                if i == j {
                    continue;
                }
                let b = b.source_reference.as_ref().unwrap();
                assert!(
                    !(a.block_index == b.block_index
                        && a.coordinate_space == b.coordinate_space
                        && a.span.byte_start < b.span.byte_end
                        && b.span.byte_start < a.span.byte_end),
                    "assigned source intervals overlap"
                );
            }
        }
        for candidate in parsed.candidates.iter().filter(|c| {
            matches!(
                c.candidate_type,
                CandidateType::Text | CandidateType::PersonName
            )
        }) {
            assert_eq!(candidate.normalized_value, Some(json!(candidate.raw_value)));
            assert!([0.3, 0.8].contains(&candidate.confidence));
        }
    }
    response
}

fn values<'a>(response: &'a ParseResponse, name: &str) -> Vec<&'a str> {
    parses(response)
        .into_iter()
        .flat_map(|p| &p.assignment.fields)
        .filter(|f| f.name == name)
        .flat_map(|f| f.candidates.iter().map(|c| c.raw_value.as_str()))
        .collect()
}

fn has_warning(response: &ParseResponse, code: &str) -> bool {
    parses(response)
        .iter()
        .any(|p| p.assignment.warnings.iter().any(|w| w.code == code))
}

fn text_candidates(response: &ParseResponse) -> Vec<&FieldCandidate> {
    parses(response)
        .into_iter()
        .flat_map(|p| &p.candidates)
        .filter(|c| {
            matches!(
                c.candidate_type,
                CandidateType::Text | CandidateType::PersonName
            )
        })
        .collect()
}

fn table(headers: &[&str], cells: Vec<RawValue>) -> RawDocument {
    let mut doc = document("");
    doc.blocks = headers
        .iter()
        .map(|h| RawValue::text(*h))
        .chain(cells)
        .enumerate()
        .map(|(i, value)| RawBlock {
            id: "repeated-id".into(),
            value,
            location: SourceLocation {
                row: Some(i / headers.len() + 1),
                column: Some(i % headers.len() + 1),
                ..SourceLocation::default()
            },
        })
        .collect();
    doc
}

#[test]
fn literal_aliases_preserve_unicode_combining_marks_and_internal_whitespace() {
    let mut name = field("person", json!("person_name"));
    name["aliases"] = json!(["姓名"]);
    let schema = profile(vec![name, field("notes", json!("text"))]);
    let input = "姓名: Zoe\u{301}  東京 O’Neil; NOTES: Call later, please";
    let response = checked(&schema, &document(input));
    assert_eq!(values(&response, "person"), ["Zoe\u{301}  東京 O’Neil"]);
    assert_eq!(values(&response, "notes"), ["Call later, please"]);
    assert!(!has_warning(&response, "text_field_ambiguous"));
    let name = text_candidates(&response)[0];
    assert_eq!(
        &input[name.source_span.byte_start..name.source_span.byte_end],
        name.raw_value
    );
    assert!(name.reasons.iter().any(|r| r.code == "caller_person_name"));
    assert!(name.reasons.iter().any(|r| r.code == "caller_label_match"));
    assert!(
        parses(&response)[0]
            .review
            .as_ref()
            .unwrap()
            .reasons
            .iter()
            .any(|r| r.code == "unrecognized_content")
    );
}

#[test]
fn header_owned_cell_outranks_inline_labels_and_keeps_source_columns() {
    for headers in [["person", "notes"], ["notes", "person"]] {
        let cells = headers
            .iter()
            .map(|header| {
                RawValue::text(if *header == "person" {
                    "  Zoë  東京  "
                } else {
                    "person: not a new assignment"
                })
            })
            .collect();
        let response = checked(
            &profile(vec![
                field("person", json!("person_name")),
                field("notes", json!("text")),
            ]),
            &table(&headers, cells),
        );
        assert_eq!(values(&response, "person"), ["Zoë  東京"]);
        assert_eq!(values(&response, "notes"), ["person: not a new assignment"]);
        let assigned = &parses(&response)[0].assignment.fields[0].candidates[0];
        assert_eq!(
            assigned.source_column,
            Some(if headers[0] == "person" { 1 } else { 2 })
        );
        assert_eq!(
            assigned.source_reference.as_ref().unwrap().span.byte_start,
            2
        );
        assert!(
            assigned
                .reasons
                .iter()
                .any(|r| r.code == "header_label_match")
        );
    }
}

#[test]
fn residual_name_and_note_hypotheses_never_assign_even_to_one_field() {
    for kinds in [
        vec!["text"],
        vec!["person_name"],
        vec!["text", "person_name"],
        vec!["person_name", "person_name"],
    ] {
        let fields = kinds
            .iter()
            .enumerate()
            .map(|(i, kind)| field(&format!("field_{i}"), json!(kind)))
            .collect();
        let response = checked(
            &profile(fields),
            &document("Ada Lovelace, please call later"),
        );
        assert!(parses(&response)[0].assignment.fields.is_empty());
        assert!(has_warning(&response, "required_field_missing"));
        assert_eq!(
            has_warning(&response, "text_field_ambiguous"),
            kinds.len() > 1
        );
        assert!(text_candidates(&response).iter().all(|c|c.confidence==0.3 && c.reasons.iter().any(|r|r.code=="residual_text")));
        assert_eq!(
            text_candidates(&response)[0].raw_value,
            "Ada Lovelace, please call later"
        );
        assert_eq!(
            parses(&response)[0].review.as_ref().unwrap().status,
            parser_core::RecordReviewStatus::NeedsReview
        );
    }
}

#[test]
fn competing_overlapping_anchors_do_not_guess_field_ownership() {
    for kinds in [
        ("text", "text"),
        ("person_name", "person_name"),
        ("text", "person_name"),
    ] {
        let fields = vec![
            field("full name", json!(kinds.0)),
            field("name", json!(kinds.1)),
        ];
        for fields in [fields.clone(), fields.into_iter().rev().collect()] {
            let response = checked(&profile(fields), &document("full name: Ada Lovelace"));
            assert!(parses(&response)[0].assignment.fields.is_empty());
            assert!(has_warning(&response, "text_field_ambiguous"));
            assert!(
                text_candidates(&response)
                    .iter()
                    .all(|c| c.raw_value == "Ada Lovelace")
            );
        }
    }
}

#[test]
fn repeated_singular_values_abstain_before_constraints_multiple_preserves_source_order() {
    let mut name = field("name", json!("person_name"));
    name["constraints"] = json!([{"kind":"maximum_length","value":3}]);
    let input = "name: Ada; name: Grace";
    let response = checked(&profile(vec![name.clone()]), &document(input));
    assert!(values(&response, "name").is_empty());
    assert!(has_warning(&response, "text_field_ambiguous"));
    name["multiple"] = json!(true);
    let response = checked(&profile(vec![name.clone()]), &document(input));
    assert_eq!(values(&response, "name"), ["Ada"]);
    name["constraints"] = json!([]);
    let response = checked(&profile(vec![name]), &document("name: Ada; name: Ada"));
    assert_eq!(values(&response, "name"), ["Ada", "Ada"]);
    assert_eq!(
        text_candidates(&response)
            .iter()
            .map(|c| c.source_span.byte_start)
            .collect::<Vec<_>>(),
        [6, 17]
    );
}

#[test]
fn scalar_overlap_keeps_fragments_unresolved_without_joining_or_truncating_assignment() {
    for (kind, value) in [
        ("email", "ada@example.test"),
        ("phone_number", "+1-202-555-0100"),
        ("integer", "42"),
        ("decimal", "12.5"),
        ("currency", "$19.95"),
        ("date", "2026/08/28"),
        ("boolean", "true"),
    ] {
        let fields = vec![
            field("name", json!("person_name")),
            field("value", json!(kind)),
        ];
        for fields in [fields.clone(), fields.into_iter().rev().collect()] {
            let response = checked(
                &profile(fields),
                &document(&format!("name: Ada {value} Lovelace")),
            );
            assert!(values(&response, "name").is_empty());
            assert_eq!(values(&response, "value"), [value]);
            assert!(has_warning(&response, "text_evidence_overlap"));
            let fragments: Vec<_> = text_candidates(&response)
                .into_iter()
                .filter(|c| c.candidate_type == CandidateType::Text)
                .map(|c| c.raw_value.as_str())
                .collect();
            assert_eq!(fragments, ["Ada", "Lovelace"]);
        }
    }
}

#[test]
fn enum_overlap_retains_typed_precedence_and_unassigned_detections_are_not_claims() {
    let enumeration = field(
        "color",
        json!({"enum":{"values":[{"value":"green","aliases":[]}]}}),
    );
    let response = checked(
        &profile(vec![field("name", json!("person_name")), enumeration]),
        &document("name: Green"),
    );
    assert!(values(&response, "name").is_empty());
    assert_eq!(values(&response, "color"), ["Green"]);
    assert!(has_warning(&response, "text_evidence_overlap"));
    let response = checked(
        &profile(vec![field("note", json!("text"))]),
        &document("note: call ada@example.test at 42"),
    );
    assert_eq!(values(&response, "note"), ["call ada@example.test at 42"]);
    assert!(
        parses(&response)[0]
            .assignment
            .unassigned_candidates
            .iter()
            .any(|c| c.candidate_type == CandidateType::Email)
    );
}

#[test]
fn directed_labels_separate_name_and_existing_types_without_reusing_evidence() {
    let schema = profile(vec![
        field("name", json!("person_name")),
        field("email", json!("email")),
        field("amount", json!("currency")),
    ]);
    let response = checked(
        &schema,
        &document("name: Ada Lovelace; email: ada@example.test; amount: $19.95"),
    );
    assert_eq!(values(&response, "name"), ["Ada Lovelace"]);
    assert_eq!(values(&response, "email"), ["ada@example.test"]);
    assert_eq!(values(&response, "amount"), ["$19.95"]);
    assert!(!has_warning(&response, "text_evidence_overlap"));
}

#[test]
fn blank_missing_label_only_and_person_name_guards_never_invent_defaults() {
    for input in [
        "",
        "  ",
        "name:",
        "name:  ",
        "name: No",
        "name: 42",
        "name: ---",
        "name: ada@example.test",
    ] {
        for required in [true, false] {
            let mut name = field("name", json!("person_name"));
            name["required"] = json!(required);
            let response = checked(&profile(vec![name]), &document(input));
            assert!(values(&response, "name").is_empty());
            assert_eq!(has_warning(&response, "required_field_missing"), required);
            assert!(text_candidates(&response).is_empty());
        }
    }
    for input in ["name: 李", "name: O'Neil", "name: Jean-Luc"] {
        let response = checked(
            &profile(vec![field("name", json!("person_name"))]),
            &document(input),
        );
        assert_eq!(values(&response, "name").len(), 1);
    }
}

#[test]
fn typed_cells_are_not_coerced_and_blank_cells_keep_original_evidence() {
    for value in [
        RawValue::Integer(42),
        RawValue::Decimal(42.5),
        RawValue::Boolean(true),
        RawValue::DateTime(45943.5),
        RawValue::Duration("PT1H".into()),
        RawValue::Error("#VALUE!".into()),
        RawValue::Null,
        RawValue::DateTimeText("2026-08-28".into()),
        RawValue::text("   "),
    ] {
        for kind in ["text", "person_name"] {
            let response = checked(
                &profile(vec![field("name", json!(kind))]),
                &table(&["name", "other"], vec![value.clone(), RawValue::Null]),
            );
            assert!(values(&response, "name").is_empty());
            assert!(text_candidates(&response).is_empty());
            assert_eq!(
                response.source_evidence.as_ref().unwrap().document.blocks[2].value,
                value
            );
            assert!(has_warning(&response, "required_field_missing"));
        }
    }
}

#[test]
fn length_constraints_count_exact_normalized_scalars_and_all_repeated_bounds() {
    for kind in ["text", "person_name"] {
        let mut name = field("name", json!(kind));
        name["constraints"] = json!([{"kind":"minimum_length","value":1},{"kind":"maximum_length","value":10},{"kind":"minimum_length","value":5},{"kind":"maximum_length","value":5}]);
        let response = checked(
            &profile(vec![name.clone()]),
            &document("name: e\u{301}  李"),
        );
        assert_eq!(values(&response, "name"), ["e\u{301}  李"]);
        name["constraints"]
            .as_array_mut()
            .unwrap()
            .push(json!({"kind":"maximum_length","value":4}));
        let response = checked(
            &profile(vec![name.clone()]),
            &document("name: e\u{301}  李"),
        );
        assert!(values(&response, "name").is_empty());
        assert_eq!(text_candidates(&response).len(), 1);
        name["constraints"] = json!([{"kind":"minimum_integer","value":0}]);
        assert_eq!(
            compile_schema_json(&profile(vec![name]).to_string())
                .unwrap_err()
                .kind,
            FailureKind::SchemaConstraintUnsupported
        );
    }
}

#[test]
fn label_boundaries_are_literal_and_do_not_add_record_or_comma_segmentation() {
    for input in ["surname: Ada", "xname: Ada", "name : Ada"] {
        let response = checked(
            &profile(vec![field("name", json!("text"))]),
            &document(input),
        );
        assert!(values(&response, "name").is_empty());
        assert_eq!(text_candidates(&response)[0].raw_value, input);
    }
    let response = checked(
        &profile(vec![field("name", json!("text"))]),
        &document("name: Lovelace, Ada"),
    );
    assert_eq!(values(&response, "name"), ["Lovelace, Ada"]);
    let response = checked(
        &profile(vec![field("name", json!("text"))]),
        &document("name: Ada\ncontinuation"),
    );
    assert_eq!(values(&response, "name"), ["Ada\ncontinuation"]);
}

#[test]
fn repeated_block_ids_and_mixed_table_exclusions_preserve_original_references() {
    let mut doc = document("name: Ada");
    doc.blocks.push(RawBlock {
        id: "record".into(),
        value: RawValue::text("name: Ada"),
        location: SourceLocation::default(),
    });
    let response = checked(&profile(vec![field("name", json!("text"))]), &doc);
    assert_eq!(
        text_candidates(&response)
            .iter()
            .map(|c| c.source_reference.as_ref().unwrap().block_index)
            .collect::<Vec<_>>(),
        [0, 1]
    );
    let mut doc = table(
        &["name", "note"],
        vec![RawValue::text("Ada"), RawValue::text("memo")],
    );
    doc.blocks.push(RawBlock {
        id: "repeated-id".into(),
        value: RawValue::text("name: excluded"),
        location: SourceLocation::default(),
    });
    let response = checked(&profile(vec![field("name", json!("text"))]), &doc);
    assert_eq!(values(&response, "name"), ["Ada"]);
    assert_eq!(
        response.source_evidence.as_ref().unwrap().blocks[4].role,
        parser_core::SourceBlockRole::Excluded
    );
}

#[test]
fn new_types_keep_strict_alternate_schema_encodings_and_datetime_failure() {
    for kind in ["text", "person_name"] {
        let ordinary = profile(vec![field("name", json!(kind))]);
        let alternate = json!(["0.1","synthetic",[["name",{kind:null},true,false,[],[]]],[true]]);
        assert_eq!(
            checked(&ordinary, &document("name: Ada")),
            checked(&alternate, &document("name: Ada"))
        );
    }
    assert_eq!(
        compile_schema_json(
            &profile(vec![
                field("name", json!("text")),
                field("time", json!("datetime"))
            ])
            .to_string()
        )
        .unwrap_err()
        .kind,
        FailureKind::SchemaFieldTypeUnsupported {
            field_type: parser_core::UnsupportedFieldType::Datetime
        }
    );
}

#[test]
fn repeated_matching_header_columns_abstain_or_assign_in_source_order() {
    for kind in ["text", "person_name"] {
        let doc = table(
            &["Name", "Name"],
            vec![
                RawValue::text("Ada Lovelace"),
                RawValue::text("Grace Hopper"),
            ],
        );
        for multiple in [false, true] {
            let mut f = field("name", json!(kind));
            f["multiple"] = json!(multiple);
            let response = checked(&profile(vec![f]), &doc);
            if multiple {
                assert_eq!(values(&response, "name"), ["Ada Lovelace", "Grace Hopper"]);
                assert_eq!(
                    parses(&response)[0].assignment.fields[0]
                        .candidates
                        .iter()
                        .map(|c| c.source_column)
                        .collect::<Vec<_>>(),
                    [Some(1), Some(2)]
                );
            } else {
                assert!(values(&response, "name").is_empty());
                assert!(has_warning(&response, "text_field_ambiguous"));
                assert!(has_warning(&response, "required_field_missing"));
            }
        }
    }
}

#[test]
fn residual_gaps_and_label_punctuation_keep_exact_unused_coverage() {
    let input = "  preface , name: Ada Lovelace; email: ada@example.test tail  ";
    let response = checked(
        &profile(vec![
            field("name", json!("person_name")),
            field("email", json!("email")),
        ]),
        &document(input),
    );
    assert_eq!(values(&response, "name"), ["Ada Lovelace"]);
    let residuals: Vec<_> = text_candidates(&response)
        .into_iter()
        .filter(|c| c.candidate_type == CandidateType::Text)
        .map(|c| c.raw_value.as_str())
        .collect();
    assert_eq!(residuals, ["preface", "tail"]);
    let unused: Vec<_> = response.source_evidence.as_ref().unwrap().blocks[0]
        .unused_spans
        .iter()
        .map(|span| &input[span.byte_start..span.byte_end])
        .collect();
    assert_eq!(unused, ["  ", " , name: ", "; email: ", " ", "  "]);
    let unassigned = &parses(&response)[0].assignment.unassigned_candidates;
    assert_eq!(unassigned.len(), 4);
    assert!(unassigned.iter().all(|c| c.confidence == 0.3));
}

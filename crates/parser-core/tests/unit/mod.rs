use super::*;

fn test_document(blocks: Vec<RawBlock>) -> RawDocument {
    RawDocument::new(
        "test-document",
        SourceMetadata {
            source_type: SourceType::Text,
            file_name: None,
            mime_type: Some("text/plain".to_owned()),
            size_bytes: None,
            delimiter: None,
        },
        blocks,
    )
}

#[test]
fn raw_document_round_trips_as_json() {
    let document = RawDocument::new(
        "document-1",
        SourceMetadata {
            source_type: SourceType::Txt,
            file_name: Some("sample.txt".to_owned()),
            mime_type: Some("text/plain".to_owned()),
            size_bytes: Some(5),
            delimiter: None,
        },
        vec![RawBlock {
            id: "block-1".to_owned(),
            value: RawValue::text("Ada  Lovelace"),
            location: SourceLocation {
                line: Some(1),
                byte_start: Some(0),
                byte_end: Some(13),
                ..SourceLocation::default()
            },
        }],
    );

    let json = serde_json::to_string(&document).expect("document should serialize");
    let decoded: RawDocument = serde_json::from_str(&json).expect("document should deserialize");

    assert_eq!(decoded, document);
    assert_eq!(
        serde_json::to_value(&document).unwrap(),
        serde_json::json!({
            "id": "document-1",
            "source": {"source_type": "txt", "file_name": "sample.txt",
                "mime_type": "text/plain", "size_bytes": 5, "delimiter": null},
            "blocks": [{"id": "block-1",
                "value": {"kind": "Text", "value": "Ada  Lovelace"},
                "location": {"line": 1, "row": null, "column": null,
                    "sheet": null, "byte_start": 0, "byte_end": 13}}],
            "warnings": []
        })
    );
}

#[test]
fn raw_document_pasted_unicode_and_absent_metadata_have_stable_json() {
    let raw = "\t  Zoë — 東京\r\n  ";
    let document = RawDocument::new(
        "pasted",
        SourceMetadata {
            source_type: SourceType::Text,
            file_name: None,
            mime_type: None,
            size_bytes: None,
            delimiter: None,
        },
        vec![RawBlock {
            id: "block".to_owned(),
            value: RawValue::text(raw),
            location: SourceLocation::default(),
        }],
    );
    let expected = serde_json::json!({
        "id": "pasted",
        "source": {"source_type": "text", "file_name": null,
            "mime_type": null, "size_bytes": null, "delimiter": null},
        "blocks": [{"id": "block", "value": {"kind": "Text", "value": raw},
            "location": {"line": null, "row": null, "column": null,
                "sheet": null, "byte_start": null, "byte_end": null}}],
        "warnings": []
    });
    let json = serde_json::to_string(&document).unwrap();
    assert_eq!(serde_json::to_value(&document).unwrap(), expected);
    assert_eq!(
        serde_json::from_str::<RawDocument>(&json).unwrap(),
        document
    );
    assert_eq!(serde_json::to_string(&document).unwrap(), json);
}

#[test]
fn raw_document_empty_round_trip_and_source_kind_tags_are_stable() {
    for (source_type, tag) in [
        (SourceType::Text, "text"),
        (SourceType::Stdin, "stdin"),
        (SourceType::Txt, "txt"),
        (SourceType::Csv, "csv"),
        (SourceType::Xlsx, "xlsx"),
    ] {
        let mut document = test_document(Vec::new());
        document.source.source_type = source_type;
        document.source.mime_type = None;
        let json = serde_json::to_string(&document).unwrap();
        let expected = format!(
            "{{\"id\":\"test-document\",\"source\":{{\"source_type\":\"{tag}\",\"file_name\":null,\"mime_type\":null,\"size_bytes\":null,\"delimiter\":null}},\"blocks\":[],\"warnings\":[]}}"
        );
        assert_eq!(json, expected);
        assert_eq!(
            serde_json::from_str::<RawDocument>(&json).unwrap(),
            document
        );
        assert_eq!(serde_json::to_string(&document).unwrap(), json);
    }
}

#[test]
fn raw_value_tags_preserve_all_typed_values() {
    let cases = [
        (
            RawValue::text("é  "),
            serde_json::json!({"kind": "Text", "value": "é  "}),
        ),
        (
            RawValue::Integer(-7),
            serde_json::json!({"kind": "Integer", "value": -7}),
        ),
        (
            RawValue::Decimal(2.5),
            serde_json::json!({"kind": "Decimal", "value": 2.5}),
        ),
        (
            RawValue::Boolean(false),
            serde_json::json!({"kind": "Boolean", "value": false}),
        ),
        (
            RawValue::DateTime(45123.5),
            serde_json::json!({"kind": "DateTime", "value": 45123.5}),
        ),
        (
            RawValue::DateTimeText("2026-08-27T12:00:00Z".to_owned()),
            serde_json::json!({"kind": "DateTimeText", "value": "2026-08-27T12:00:00Z"}),
        ),
        (
            RawValue::Duration("PT1H".to_owned()),
            serde_json::json!({"kind": "Duration", "value": "PT1H"}),
        ),
        (
            RawValue::Error("#VALUE!".to_owned()),
            serde_json::json!({"kind": "Error", "value": "#VALUE!"}),
        ),
        (RawValue::Null, serde_json::json!({"kind": "Null"})),
    ];
    for (value, expected) in cases {
        assert_eq!(serde_json::to_value(&value).unwrap(), expected);
        assert_eq!(serde_json::from_value::<RawValue>(expected).unwrap(), value);
    }
}

#[test]
fn parse_contract_0_1_golden_round_trips_without_rewriting_fields() {
    // Captured before adding source evidence, from the real CLI with stdin
    // "ada@example.test 42" and fixtures/schema/contact.json.
    let json = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/contracts/parse-0.1.json"
    ));
    let response: ParseResponse = serde_json::from_str(json).unwrap();
    assert!(response.source_evidence.is_none());
    let ParseContent::Text { records } = &response.content else {
        panic!("text fixture")
    };
    assert!(records[0].parse.review.is_none());
    assert!(
        records[0]
            .parse
            .candidates
            .iter()
            .all(|candidate| candidate.source_reference.is_none())
    );
    assert_eq!(
        serde_json::to_value(response).unwrap(),
        serde_json::from_str::<serde_json::Value>(json).unwrap()
    );
}

fn response_parses(response: &ParseResponse) -> Vec<&TextParseResult> {
    match &response.content {
        ParseContent::Text { records } => records.iter().map(|record| &record.parse).collect(),
        ParseContent::Table { sheets } => sheets
            .iter()
            .flat_map(|sheet| sheet.records.iter().map(|record| &record.parse))
            .collect(),
    }
}

fn assert_complete_source_evidence(response: &ParseResponse) {
    let evidence = response.source_evidence.as_ref().expect("source evidence");
    let document = &evidence.document;
    let mut covered: Vec<Vec<bool>> = document
        .blocks
        .iter()
        .map(|block| vec![false; block.value.to_text().len()])
        .collect();
    for parse in response_parses(response) {
        for candidate in parse
            .candidates
            .iter()
            .chain(
                parse
                    .assignment
                    .fields
                    .iter()
                    .flat_map(|field| &field.candidates),
            )
            .chain(&parse.assignment.unassigned_candidates)
        {
            let reference = candidate
                .source_reference
                .as_ref()
                .expect("every public candidate copy has a reference");
            assert_eq!(
                reference.resolve(document).as_deref(),
                Some(candidate.raw_value.as_str())
            );
            assert!(
                parse
                    .candidates
                    .iter()
                    .any(
                        |detected| detected.source_reference == candidate.source_reference
                            && detected.candidate_type == candidate.candidate_type
                    )
            );
            for byte in &mut covered[reference.block_index]
                [reference.span.byte_start..reference.span.byte_end]
            {
                *byte = true;
            }
        }
    }
    assert_eq!(evidence.blocks.len(), document.blocks.len());
    for (index, coverage) in evidence.blocks.iter().enumerate() {
        assert_eq!(coverage.block_index, index);
        let block = &document.blocks[index];
        assert_eq!(
            coverage.coordinate_space,
            SourceCoordinateSpace::for_value(&block.value)
        );
        if coverage.role == SourceBlockRole::Parsed {
            for span in &coverage.unused_spans {
                assert!(
                    block
                        .value
                        .to_text()
                        .get(span.byte_start..span.byte_end)
                        .is_some()
                );
                for byte in &mut covered[index][span.byte_start..span.byte_end] {
                    assert!(
                        !*byte,
                        "unused content must not overlap candidates or other unused spans"
                    );
                    *byte = true;
                }
            }
            assert!(
                covered[index].iter().all(|byte| *byte),
                "every source byte must be accounted for"
            );
        } else {
            assert!(coverage.reason.is_some());
            assert!(coverage.unused_spans.is_empty());
        }
    }
}

#[test]
fn source_evidence_retains_unrecognized_unicode_blanks_and_warning_scopes() {
    let raw = "  Zoë — 東京  ";
    let mut document = test_document(vec![
        RawBlock {
            id: "note".to_owned(),
            value: RawValue::text(raw),
            location: SourceLocation::default(),
        },
        RawBlock {
            id: "blank".to_owned(),
            value: RawValue::text(""),
            location: SourceLocation::default(),
        },
    ]);
    document.warnings.push(ParserWarning {
        code: "synthetic_input_warning".to_owned(),
        message: "synthetic extraction warning".to_owned(),
        location: Some(SourceLocation::default()),
    });
    let fields = [field_of("email", &[], CandidateType::Email, true, false)];
    let response = parse_document_with_assignment(&document, &fields, &[], None);
    let evidence = response.source_evidence.as_ref().unwrap();
    assert_eq!(evidence.document, document);
    assert_eq!(response.warnings, document.warnings);
    assert_eq!(
        evidence.blocks[0].unused_spans,
        vec![TextSpan {
            byte_start: 0,
            byte_end: raw.len()
        }]
    );
    assert_eq!(
        evidence.blocks[1].unused_spans,
        vec![TextSpan {
            byte_start: 0,
            byte_end: 0
        }]
    );
    let parses = response_parses(&response);
    assert_eq!(parses.len(), 2);
    for parse in &parses {
        assert_eq!(parse.assignment.warnings[0].code, "required_field_missing");
        assert_eq!(
            parse.review.as_ref().unwrap().status,
            RecordReviewStatus::NeedsReview
        );
    }
    assert_eq!(
        parses[0]
            .review
            .as_ref()
            .unwrap()
            .reasons
            .iter()
            .map(|reason| reason.code.as_str())
            .collect::<Vec<_>>(),
        vec![
            "no_candidates",
            "assignment_warnings",
            "unrecognized_content"
        ]
    );
    assert_complete_source_evidence(&response);
    assert_eq!(
        serde_json::to_string(&response).unwrap(),
        serde_json::to_string(&parse_document_with_assignment(
            &document,
            &fields,
            &[],
            None
        ))
        .unwrap()
    );
}

#[test]
fn source_references_cover_detected_assigned_unassigned_and_overlapping_candidates() {
    let document = test_document(vec![
        RawBlock {
            id: "duplicate-id".to_owned(),
            value: RawValue::text("Zoë ada@example.test 42"),
            location: SourceLocation::default(),
        },
        RawBlock {
            id: "duplicate-id".to_owned(),
            value: RawValue::text("+15550101"),
            location: SourceLocation::default(),
        },
    ]);
    let fields = [field_of("email", &[], CandidateType::Email, false, false)];
    let response = parse_document_with_assignment(&document, &fields, &[], None);
    assert_complete_source_evidence(&response);
    let parses = response_parses(&response);
    let email = &parses[0].assignment.fields[0].candidates[0];
    assert_eq!(
        email.source_span,
        TextSpan {
            byte_start: 5,
            byte_end: 21
        }
    );
    assert_eq!(
        email.source_reference.as_ref().unwrap().span,
        email.source_span
    );
    assert_eq!(parses[0].assignment.unassigned_candidates.len(), 1);
    assert!(
        parses[1].candidates.len() > 1,
        "overlapping integer/phone evidence"
    );
    for candidate in &parses[1].candidates {
        assert_eq!(candidate.source_reference.as_ref().unwrap().block_index, 1);
    }
}

#[test]
fn table_evidence_retains_headers_typed_blanks_exclusions_and_original_cell_offsets() {
    let mut document = table_document(vec![
        table_block(Some("Sheet"), 2, 2, RawValue::Integer(42)),
        table_block(Some("Sheet"), 1, 1, RawValue::text(" Email ")),
        table_block(Some("Sheet"), 1, 2, RawValue::text("Count")),
        table_block(Some("Sheet"), 1, 3, RawValue::text("Note")),
        table_block(Some("Sheet"), 1, 4, RawValue::text("Extra")),
        table_block(
            Some("Sheet"),
            2,
            1,
            RawValue::text("\u{2003} Zoë ada@example.test  "),
        ),
        table_block(Some("Sheet"), 2, 3, RawValue::Null),
        table_block(Some("Sheet"), 2, 4, RawValue::text("  ")),
        RawBlock {
            id: "orphan".to_owned(),
            value: RawValue::text("kept outside table"),
            location: SourceLocation::default(),
        },
    ]);
    document.source.source_type = SourceType::Xlsx;
    for block in &mut document.blocks {
        block.id = "repeated-id".to_owned();
    }
    document.warnings.push(ParserWarning {
        code: "synthetic_input_warning".to_owned(),
        message: "retain me".to_owned(),
        location: None,
    });
    let fields = [field_of("email", &[], CandidateType::Email, true, false)];
    let response = parse_document_with_assignment(&document, &fields, &[], None);
    assert_complete_source_evidence(&response);
    let evidence = response.source_evidence.as_ref().unwrap();
    assert_eq!(evidence.document, document);
    assert_eq!(evidence.blocks[1].role, SourceBlockRole::Header);
    assert_eq!(evidence.blocks[8].role, SourceBlockRole::Excluded);
    assert_eq!(
        evidence.blocks[8].reason.as_ref().unwrap().code,
        "row_provenance_missing"
    );
    assert_eq!(
        response
            .warnings
            .iter()
            .map(|warning| warning.code.as_str())
            .collect::<Vec<_>>(),
        vec!["synthetic_input_warning", "row_provenance_missing"]
    );
    let parses = response_parses(&response);
    assert_eq!(parses.len(), 1);
    let email = &parses[0].assignment.fields[0].candidates[0];
    assert_eq!(
        email.source_span,
        TextSpan {
            byte_start: 5,
            byte_end: 21
        }
    );
    assert_eq!(
        email.source_reference.as_ref().unwrap(),
        &SourceReference {
            block_index: 5,
            coordinate_space: SourceCoordinateSpace::RawTextUtf8,
            span: TextSpan {
                byte_start: 9,
                byte_end: 25
            }
        }
    );
    assert!(
        email
            .reasons
            .iter()
            .any(|reason| reason.code == "header_label_match")
    );
    let integer = &parses[0].assignment.unassigned_candidates[0];
    assert_eq!(
        integer.source_reference.as_ref().unwrap(),
        &SourceReference {
            block_index: 0,
            coordinate_space: SourceCoordinateSpace::RenderedValueUtf8,
            span: TextSpan {
                byte_start: 0,
                byte_end: 2
            }
        }
    );
    assert_eq!(evidence.document.blocks[0].value, RawValue::Integer(42));
    assert_eq!(evidence.document.blocks[0].location.byte_start, None);
    assert_eq!(evidence.document.blocks[6].value, RawValue::Null);
    assert_eq!(
        evidence.blocks[7].unused_spans,
        vec![TextSpan {
            byte_start: 0,
            byte_end: 2
        }]
    );
}

#[test]
fn review_is_deterministic_and_never_automatic_approval() {
    let fields = [field_of("email", &[], CandidateType::Email, true, false)];
    let draft = parse_text_with_assignment("ada@example.test", &fields, &[]);
    assert_eq!(
        draft.review.as_ref().unwrap(),
        &RecordReview {
            status: RecordReviewStatus::Draft,
            reasons: vec![]
        }
    );
    let ambiguous =
        parse_text_with_assignment("ada@example.test grace@example.test unknown", &fields, &[]);
    assert_eq!(
        ambiguous.assignment.warnings[0].code,
        "multiple_candidates_ambiguous"
    );
    assert_eq!(
        ambiguous.review.as_ref().unwrap().status,
        RecordReviewStatus::NeedsReview
    );
    assert_eq!(
        ambiguous
            .review
            .as_ref()
            .unwrap()
            .reasons
            .iter()
            .map(|reason| reason.code.as_str())
            .collect::<Vec<_>>(),
        vec![
            "assignment_warnings",
            "unassigned_candidates",
            "unrecognized_content"
        ]
    );
    assert!(
        draft.candidates[0].source_reference.is_none(),
        "standalone text has no canonical document"
    );
}

#[test]
fn source_reference_rejects_invalid_coordinates_without_panicking() {
    let document = test_document(vec![RawBlock {
        id: "unicode".to_owned(),
        value: RawValue::text("é"),
        location: SourceLocation::default(),
    }]);
    let mut reference = SourceReference {
        block_index: 0,
        coordinate_space: SourceCoordinateSpace::RawTextUtf8,
        span: TextSpan {
            byte_start: 0,
            byte_end: 2,
        },
    };
    assert_eq!(reference.resolve(&document).as_deref(), Some("é"));
    reference.span.byte_start = 1;
    assert_eq!(reference.resolve(&document), None);
    reference.span.byte_start = 0;
    reference.span.byte_end = 3;
    assert_eq!(reference.resolve(&document), None);
    reference.span.byte_end = 2;
    reference.coordinate_space = SourceCoordinateSpace::RenderedValueUtf8;
    assert_eq!(reference.resolve(&document), None);
    reference.block_index = 1;
    assert_eq!(reference.resolve(&document), None);
}

#[test]
fn empty_document_embeds_empty_evidence_instead_of_legacy_absence() {
    let document = test_document(vec![]);
    let response = parse_document_with_assignment(&document, &[], &[], None);
    assert_eq!(
        response.source_evidence.as_ref().unwrap().document,
        document
    );
    assert_complete_source_evidence(&response);
    assert!(response.source_evidence.as_ref().unwrap().blocks.is_empty());
}

#[test]
fn email_detection_preserves_value_and_byte_span() {
    let text = "Contact: Ada ada@example.test.";
    let candidates = detect_email_candidates(text);

    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].candidate_type, CandidateType::Email);
    assert_eq!(candidates[0].raw_value, "ada@example.test");
    assert_eq!(
        &text[candidates[0].source_span.byte_start..candidates[0].source_span.byte_end],
        "ada@example.test"
    );
    assert_eq!(
        candidates[0].normalized_value,
        Some(serde_json::Value::String("ada@example.test".to_owned()))
    );
}

#[test]
fn email_detection_ignores_near_misses() {
    assert!(detect_email_candidates("missing-at.example invalid@localhost").is_empty());
}

#[test]
fn integer_detection_returns_normalized_values_and_spans() {
    let text = "count: -42, next 7.";
    let candidates = detect_integer_candidates(text);

    assert_eq!(candidates.len(), 2);
    assert_eq!(candidates[0].raw_value, "-42");
    assert_eq!(candidates[0].normalized_value, Some(serde_json::json!(-42)));
    assert_eq!(
        &text[candidates[0].source_span.byte_start..candidates[0].source_span.byte_end],
        "-42"
    );
    assert_eq!(candidates[1].raw_value, "7");
}

#[test]
fn integer_detection_does_not_extract_digits_from_mixed_tokens() {
    assert!(detect_integer_candidates("phone 555-0123 room12").is_empty());
}

#[test]
fn decimal_detection_excludes_integers_and_normalizes_values() {
    let text = "whole 7 decimal -12.50, invalid 1.2.3";
    let candidates = detect_decimal_candidates(text);

    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].candidate_type, CandidateType::Decimal);
    assert_eq!(candidates[0].raw_value, "-12.50");
    assert_eq!(
        candidates[0].normalized_value,
        Some(serde_json::json!(-12.5))
    );
}

#[test]
fn phone_detection_normalizes_separators_and_preserves_span() {
    let text = "call +1-555-012-3456.";
    let candidates = detect_phone_candidates(text);

    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].candidate_type, CandidateType::PhoneNumber);
    assert_eq!(candidates[0].raw_value, "+1-555-012-3456");
    assert_eq!(
        candidates[0].normalized_value,
        Some(serde_json::json!("15550123456"))
    );
    assert_eq!(
        &text[candidates[0].source_span.byte_start..candidates[0].source_span.byte_end],
        "+1-555-012-3456"
    );
}

#[test]
fn phone_detection_ignores_short_and_mixed_tokens() {
    assert!(detect_phone_candidates("room 12345 code A5550123").is_empty());
}

#[test]
fn currency_detection_normalizes_symbol_amounts_and_preserves_span() {
    let text = "Total: $12.50, other EUR 9.00";
    let candidates = detect_currency_candidates(text);

    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].candidate_type, CandidateType::Currency);
    assert_eq!(candidates[0].raw_value, "$12.50");
    assert_eq!(
        candidates[0].normalized_value,
        Some(serde_json::json!(12.5))
    );
    assert_eq!(
        &text[candidates[0].source_span.byte_start..candidates[0].source_span.byte_end],
        "$12.50"
    );
}

#[test]
fn currency_detection_ignores_unmarked_amounts() {
    assert!(detect_currency_candidates("amount 12.50 dollars").is_empty());
}

#[test]
fn enum_detection_normalizes_aliases_to_canonical_values() {
    let definitions = vec![(
        "active".to_owned(),
        vec!["enabled".to_owned(), "on".to_owned()],
    )];
    let text = "Status: ENABLED.";
    let candidates = detect_enum_candidates(text, &definitions);

    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].candidate_type, CandidateType::Enum);
    assert_eq!(candidates[0].raw_value, "ENABLED");
    assert_eq!(
        candidates[0].normalized_value,
        Some(serde_json::json!("active"))
    );
    assert_eq!(
        &text[candidates[0].source_span.byte_start..candidates[0].source_span.byte_end],
        "ENABLED"
    );
}

#[test]
fn enum_detection_ignores_values_without_definitions() {
    let definitions = vec![("active".to_owned(), vec!["enabled".to_owned()])];
    assert!(detect_enum_candidates("pending unknown", &definitions).is_empty());
}

#[test]
fn assignment_selects_highest_confidence_compatible_candidate() {
    let mut candidates = detect_email_candidates("first a@example.test second b@example.test");
    candidates[0].confidence = 0.8;
    let result = assign_candidates(
        "first a@example.test second b@example.test",
        &candidates,
        &[AssignmentField {
            name: "email".to_owned(),
            aliases: Vec::new(),
            candidate_type: CandidateType::Email,
            required: true,
            multiple: false,
            unique: true,
            constraints: Vec::new(),
            expected_column: None,
        }],
    );

    assert_eq!(result.fields.len(), 1);
    assert_eq!(result.fields[0].candidates[0].raw_value, "b@example.test");
    assert_eq!(result.unassigned_candidates.len(), 1);
    assert_eq!(result.warnings[0].code, "multiple_candidates_ambiguous");
}

#[test]
fn assignment_prefers_nearby_field_label_over_confidence_alone() {
    let text = "backup a@example.test Email: b@example.test";
    let mut candidates = detect_email_candidates(text);
    candidates[1].confidence = 0.8;
    let result = assign_candidates(
        text,
        &candidates,
        &[AssignmentField {
            name: "email".to_owned(),
            aliases: vec!["contact".to_owned()],
            candidate_type: CandidateType::Email,
            required: true,
            multiple: false,
            unique: true,
            constraints: Vec::new(),
            expected_column: None,
        }],
    );

    assert_eq!(result.fields[0].candidates[0].raw_value, "b@example.test");
}

fn assert_context_assignment_evidence(prefix: &str, multiple_candidates: bool) {
    let suffix = if multiple_candidates {
        " ada@example.test grace@example.test"
    } else {
        " ada@example.test"
    };
    let text = format!("{prefix}{suffix}");
    let document = test_document(vec![RawBlock {
        id: "unicode-context".to_owned(),
        value: RawValue::text(&text),
        location: SourceLocation::default(),
    }]);
    let fields = [field_of("email", &[], CandidateType::Email, true, false)];
    let response = parse_document_with_assignment(&document, &fields, &[], None);
    assert_eq!(
        response.source_evidence.as_ref().unwrap().document,
        document
    );
    assert_eq!(response.warnings, document.warnings);
    assert_complete_source_evidence(&response);
    let parses = response_parses(&response);
    assert_eq!(parses.len(), 1);
    let parse = parses[0];
    assert_eq!(
        parse.candidates.len(),
        if multiple_candidates { 2 } else { 1 }
    );
    assert_eq!(
        parse.assignment.fields[0].candidates,
        vec![parse.candidates[0].clone()]
    );
    assert_eq!(
        parse.assignment.fields[0].candidates[0].raw_value,
        "ada@example.test"
    );
    assert_eq!(
        parse.assignment.unassigned_candidates,
        parse.candidates[1..]
    );
    for candidate in &parse.candidates {
        let start = text.find(&candidate.raw_value).unwrap();
        let expected_span = TextSpan {
            byte_start: start,
            byte_end: start + candidate.raw_value.len(),
        };
        assert_eq!(candidate.source_span, expected_span);
        assert_eq!(
            candidate.normalized_value,
            Some(serde_json::json!(candidate.raw_value))
        );
        assert_eq!(
            candidate.source_reference,
            Some(SourceReference {
                block_index: 0,
                coordinate_space: SourceCoordinateSpace::RawTextUtf8,
                span: expected_span,
            })
        );
    }
    let warning_codes: Vec<_> = parse
        .assignment
        .warnings
        .iter()
        .map(|warning| warning.code.as_str())
        .collect();
    assert_eq!(
        warning_codes,
        if multiple_candidates {
            vec!["multiple_candidates_ambiguous"]
        } else {
            vec![]
        }
    );
    let review = parse.review.as_ref().unwrap();
    assert_eq!(review.status, RecordReviewStatus::NeedsReview);
    let reason_codes: Vec<_> = review
        .reasons
        .iter()
        .map(|reason| reason.code.as_str())
        .collect();
    assert_eq!(
        reason_codes,
        if multiple_candidates {
            vec![
                "assignment_warnings",
                "unassigned_candidates",
                "unrecognized_content",
            ]
        } else {
            vec!["unrecognized_content"]
        }
    );
    assert_eq!(
        serde_json::to_string(&response).unwrap(),
        serde_json::to_string(&parse_document_with_assignment(
            &document,
            &fields,
            &[],
            None
        ))
        .unwrap()
    );
}

#[test]
fn assignment_context_two_byte_prefix_preserves_evidence() {
    // First email starts at byte 43; the 40-byte window starts inside é at byte 3.
    assert_context_assignment_evidence(&"é".repeat(21), true);
}

#[test]
fn assignment_context_three_byte_prefix_preserves_evidence() {
    assert_context_assignment_evidence(&"東京".repeat(15), true);
}

#[test]
fn assignment_context_four_byte_prefix_preserves_evidence() {
    assert_context_assignment_evidence(&"😀".repeat(11), true);
}

#[test]
fn assignment_context_ascii_and_single_candidate_controls() {
    assert_context_assignment_evidence(&"x".repeat(42), true);
    for prefix in ["é".repeat(21), "東京".repeat(15), "😀".repeat(11)] {
        // max_by does not score a lone candidate, so these passed before #21.
        assert_context_assignment_evidence(&prefix, false);
    }
}

#[test]
fn assignment_context_window_keeps_only_complete_labels_within_40_bytes() {
    for label in ["EMAIL", "é", "東", "😀"] {
        let field = field_of("email", &[label], CandidateType::Email, true, false);
        // At extra=0 the complete label is exactly inside the window. Moving
        // it out by 1..=len bytes covers every boundary inside a UTF-8 scalar.
        for extra in 0..=label.len() {
            let padding = " ".repeat(40 - label.len() - 1 + extra);
            let text = format!("backup ada@example.test {label}:{padding}grace@example.test");
            let mut candidates = detect_email_candidates(&text);
            assert_eq!(candidates.len(), 2);
            candidates[1].confidence = 0.8;
            assert_eq!(
                candidate_score(&text, &candidates[1], &field, None).1,
                extra == 0
            );
            let result = assign_candidates(&text, &candidates, std::slice::from_ref(&field));
            let selected = if extra == 0 { 1 } else { 0 };
            assert_eq!(
                result.fields[0].candidates,
                vec![candidates[selected].clone()]
            );
            assert_eq!(
                result.unassigned_candidates,
                vec![candidates[1 - selected].clone()]
            );
            assert_eq!(result.warnings[0].code, "multiple_candidates_ambiguous");
            assert_eq!(
                result,
                assign_candidates(&text, &candidates, std::slice::from_ref(&field))
            );
        }
    }
}

#[test]
fn table_assignment_context_preserves_unicode_and_cell_references() {
    for prefix in ["é".repeat(21), "東京".repeat(15), "😀".repeat(11)] {
        let document = table_document(vec![
            table_block(None, 1, 1, RawValue::text(format!("  {prefix}  "))),
            table_block(None, 1, 2, RawValue::text(" ada@example.test ")),
            table_block(None, 1, 3, RawValue::text(" grace@example.test ")),
        ]);
        let fields = [field_of("email", &[], CandidateType::Email, true, false)];
        let response = parse_document_with_assignment(&document, &fields, &[], None);
        assert_eq!(
            response.source_evidence.as_ref().unwrap().document,
            document
        );
        assert_complete_source_evidence(&response);
        let ParseContent::Table { sheets } = &response.content else {
            panic!("row provenance must use table assignment");
        };
        assert_eq!(sheets[0].records.len(), 1);
        let parse = &sheets[0].records[0].parse;
        assert_eq!(parse.candidates.len(), 2);
        assert_eq!(
            parse.assignment.fields[0].candidates,
            vec![parse.candidates[0].clone()]
        );
        assert_eq!(
            parse.assignment.fields[0].candidates[0].raw_value,
            "ada@example.test"
        );
        assert_eq!(
            parse.assignment.unassigned_candidates,
            vec![parse.candidates[1].clone()]
        );
        assert_eq!(
            parse.assignment.warnings[0].code,
            "multiple_candidates_ambiguous"
        );
        assert_eq!(
            parse.review.as_ref().unwrap().status,
            RecordReviewStatus::NeedsReview
        );
        let row_text = format!("{prefix} ada@example.test grace@example.test");
        for (index, candidate) in parse.candidates.iter().enumerate() {
            let start = row_text.find(&candidate.raw_value).unwrap();
            assert_eq!(
                candidate.source_span,
                TextSpan {
                    byte_start: start,
                    byte_end: start + candidate.raw_value.len()
                }
            );
            assert_eq!(candidate.source_column, Some(index + 2));
            assert_eq!(
                candidate.source_reference,
                Some(SourceReference {
                    block_index: index + 1,
                    coordinate_space: SourceCoordinateSpace::RawTextUtf8,
                    span: TextSpan {
                        byte_start: 1,
                        byte_end: 1 + candidate.raw_value.len()
                    },
                })
            );
        }
        assert_eq!(
            response,
            parse_document_with_assignment(&document, &fields, &[], None)
        );
    }
}

#[test]
fn assignment_prefers_expected_column_context_over_confidence_alone() {
    let text = "first a@example.test second b@example.test";
    let mut candidates = detect_email_candidates(text);
    candidates[0].source_column = Some(1);
    candidates[1].source_column = Some(2);
    candidates[1].confidence = 0.8;
    let result = assign_candidates(
        text,
        &candidates,
        &[AssignmentField {
            name: "email".to_owned(),
            aliases: Vec::new(),
            candidate_type: CandidateType::Email,
            required: true,
            multiple: false,
            unique: true,
            constraints: Vec::new(),
            expected_column: Some(2),
        }],
    );

    assert_eq!(result.fields[0].candidates[0].raw_value, "b@example.test");
}

#[test]
fn assignment_reports_missing_required_and_unassigned_candidates() {
    let candidates = detect_integer_candidates("count 4");
    let result = assign_candidates(
        "count 4",
        &candidates,
        &[AssignmentField {
            name: "email".to_owned(),
            aliases: Vec::new(),
            candidate_type: CandidateType::Email,
            required: true,
            multiple: false,
            unique: true,
            constraints: Vec::new(),
            expected_column: None,
        }],
    );

    assert!(result.fields.is_empty());
    assert_eq!(result.unassigned_candidates.len(), 1);
    assert_eq!(result.warnings[0].code, "required_field_missing");
}

#[test]
fn assignment_keeps_all_compatible_candidates_for_multiple_fields() {
    let candidates = detect_integer_candidates("first 4 second 7");
    let result = assign_candidates(
        "first 4 second 7",
        &candidates,
        &[AssignmentField {
            name: "counts".to_owned(),
            aliases: Vec::new(),
            candidate_type: CandidateType::Integer,
            required: false,
            multiple: true,
            unique: false,
            constraints: Vec::new(),
            expected_column: None,
        }],
    );

    assert_eq!(result.fields[0].candidates.len(), 2);
    assert!(result.warnings.is_empty());
    assert!(result.unassigned_candidates.is_empty());
}

#[test]
fn assignment_filters_candidates_that_violate_integer_constraints() {
    let text = "small 3 valid 12";
    let candidates = detect_integer_candidates(text);
    let result = assign_candidates(
        text,
        &candidates,
        &[AssignmentField {
            name: "quantity".to_owned(),
            aliases: Vec::new(),
            candidate_type: CandidateType::Integer,
            required: true,
            multiple: false,
            unique: true,
            constraints: vec![AssignmentConstraint::MinimumInteger(10)],
            expected_column: None,
        }],
    );

    assert_eq!(result.fields[0].candidates[0].raw_value, "12");
    assert_eq!(result.unassigned_candidates[0].raw_value, "3");
}

#[test]
fn assignment_reports_missing_required_when_all_candidates_violate_constraints() {
    let text = "quantity 3";
    let candidates = detect_integer_candidates(text);
    let result = assign_candidates(
        text,
        &candidates,
        &[AssignmentField {
            name: "quantity".to_owned(),
            aliases: Vec::new(),
            candidate_type: CandidateType::Integer,
            required: true,
            multiple: false,
            unique: true,
            constraints: vec![AssignmentConstraint::MinimumInteger(10)],
            expected_column: None,
        }],
    );

    assert!(result.fields.is_empty());
    assert_eq!(result.unassigned_candidates.len(), 1);
    assert_eq!(result.warnings[0].code, "required_field_missing");
}

#[test]
fn assignment_models_round_trip_through_json() {
    let field = AssignmentField {
        name: "quantity".to_owned(),
        aliases: vec!["count".to_owned()],
        candidate_type: CandidateType::Integer,
        required: true,
        multiple: false,
        unique: true,
        constraints: vec![AssignmentConstraint::MinimumInteger(1)],
        expected_column: None,
    };
    let result = assign_candidates(
        "quantity: 4",
        &detect_integer_candidates("quantity: 4"),
        &[field],
    );
    let json = serde_json::to_string(&result).expect("assignment should serialize");
    let decoded: AssignmentResult =
        serde_json::from_str(&json).expect("assignment should deserialize");

    assert_eq!(decoded, result);
}

#[test]
fn text_pipeline_detects_and_assigns_caller_defined_fields() {
    let fields = vec![
        AssignmentField {
            name: "email".to_owned(),
            aliases: Vec::new(),
            candidate_type: CandidateType::Email,
            required: true,
            multiple: false,
            unique: true,
            constraints: Vec::new(),
            expected_column: None,
        },
        AssignmentField {
            name: "status".to_owned(),
            aliases: Vec::new(),
            candidate_type: CandidateType::Enum,
            required: true,
            multiple: false,
            unique: true,
            constraints: Vec::new(),
            expected_column: None,
        },
    ];
    let result = parse_text_with_assignment(
        "Email: ada@example.test Status: ENABLED",
        &fields,
        &[("active".to_owned(), vec!["enabled".to_owned()])],
    );

    assert_eq!(result.candidates.len(), 2);
    assert_eq!(result.assignment.fields.len(), 2);
    assert!(result.assignment.unassigned_candidates.is_empty());
    assert!(result.assignment.warnings.is_empty());
}

#[test]
fn boolean_detection_normalizes_common_aliases() {
    let candidates = detect_boolean_candidates("Enabled: YES disabled: off maybe");

    assert_eq!(candidates.len(), 2);
    assert_eq!(candidates[0].raw_value, "YES");
    assert_eq!(
        candidates[0].normalized_value,
        Some(serde_json::json!(true))
    );
    assert_eq!(candidates[1].raw_value, "off");
    assert_eq!(
        candidates[1].normalized_value,
        Some(serde_json::json!(false))
    );
}

#[test]
fn boolean_detection_ignores_embedded_aliases() {
    assert!(detect_boolean_candidates("yesterday onboard truthful").is_empty());
}

#[test]
fn date_detection_normalizes_supported_formats() {
    let text = "started 2026-08-23, renewed 2027/01/05";
    let candidates = detect_date_candidates(text);

    assert_eq!(candidates.len(), 2);
    assert_eq!(candidates[0].raw_value, "2026-08-23");
    assert_eq!(
        candidates[1].normalized_value,
        Some(serde_json::json!("2027-01-05"))
    );
}

#[test]
fn date_detection_rejects_invalid_calendar_values() {
    assert!(detect_date_candidates("2026-02-29 2026-13-01 26-01-01").is_empty());
}

#[test]
fn parser_errors_have_stable_codes() {
    let error = ParserError::InvalidUtf8 {
        path: "input.txt".to_owned(),
        valid_up_to: 4,
    };

    assert_eq!(error.code(), "invalid_utf8");
    assert_eq!(
        error.to_string(),
        "input.txt is not valid UTF-8 at byte offset 4"
    );
}

#[test]
fn input_limits_have_stable_codes() {
    let error = ParserError::LineTooLong {
        source: "<stdin>".to_owned(),
        line: 3,
        limit: 10,
        actual: 11,
    };

    assert_eq!(error.code(), "line_too_long");
    assert_eq!(
        error.to_string(),
        "<stdin> line 3 exceeds the 10-byte limit (11 bytes)"
    );
}

#[test]
fn normalization_preserves_raw_value_and_records_transforms() {
    let block = RawBlock {
        id: "block-1".to_owned(),
        value: RawValue::text("  Ada  —  “Lovelace”\r\n"),
        location: SourceLocation::default(),
    };

    let normalized = normalize_block(&block);

    assert_eq!(normalized.source_block_id, "block-1");
    assert_eq!(normalized.original, block.value);
    assert_eq!(normalized.normalized_text, "Ada - \"Lovelace\"");
    assert_eq!(
        normalized.transformations,
        vec![
            Transformation::LineEndingsNormalized,
            Transformation::DashesNormalized,
            Transformation::QuotesNormalized,
            Transformation::WhitespaceTrimmed,
            Transformation::WhitespaceCollapsed,
        ]
    );

    let json = serde_json::to_string(&normalized).expect("normalized block should serialize");
    let decoded: NormalizedBlock =
        serde_json::from_str(&json).expect("normalized block should deserialize");
    assert_eq!(decoded, normalized);
}

#[test]
fn normalization_marks_noise_without_removing_it() {
    let cases = [
        ("- item", Transformation::ListMarkerDetected),
        ("12. item", Transformation::ListMarkerDetected),
        ("[12:30] Alice", Transformation::TimestampPrefixDetected),
        ("[Alice]: value", Transformation::SenderPrefixDetected),
        ("# Heading", Transformation::HeadingDetected),
    ];

    for (value, expected) in cases {
        let block = RawBlock {
            id: value.to_owned(),
            value: RawValue::text(value),
            location: SourceLocation::default(),
        };
        let normalized = normalize_block(&block);

        assert_eq!(normalized.normalized_text, value);
        assert!(normalized.transformations.contains(&expected));
    }
}

#[test]
fn normalization_options_can_disable_derived_changes() {
    let block = RawBlock {
        id: "block-1".to_owned(),
        value: RawValue::text("  Ada  —  Lovelace  "),
        location: SourceLocation::default(),
    };
    let options = NormalizationOptions {
        normalize_line_endings: false,
        trim_whitespace: false,
        collapse_whitespace: false,
        normalize_punctuation: false,
        mark_noise: false,
    };

    let normalized = normalize_block_with_options(&block, &options);

    assert_eq!(normalized.normalized_text, "  Ada  —  Lovelace  ");
    assert!(normalized.transformations.is_empty());
    assert_eq!(normalized.original, block.value);
}

#[test]
fn normalization_converts_typed_values_without_replacing_originals() {
    let block = RawBlock {
        id: "number".to_owned(),
        value: RawValue::Integer(42),
        location: SourceLocation::default(),
    };

    let normalized = normalize_block(&block);

    assert_eq!(normalized.normalized_text, "42");
    assert_eq!(normalized.original, RawValue::Integer(42));
}

#[test]
fn one_block_strategy_produces_traceable_candidates() {
    let document = test_document(vec![
        RawBlock {
            id: "block-1".to_owned(),
            value: RawValue::text("Ada"),
            location: SourceLocation {
                line: Some(1),
                ..SourceLocation::default()
            },
        },
        RawBlock {
            id: "block-2".to_owned(),
            value: RawValue::text("Grace"),
            location: SourceLocation {
                line: Some(2),
                ..SourceLocation::default()
            },
        },
    ]);

    let candidates = segment_document(&document, &SegmentationOptions::default());

    assert_eq!(candidates.len(), 2);
    assert_eq!(candidates[0].id, "record-1");
    assert_eq!(candidates[0].source_block_ids, vec!["block-1"]);
    assert_eq!(candidates[0].text, "Ada");
    assert_eq!(candidates[0].confidence, 1.0);
    assert_eq!(candidates[0].reasons[0].code, "one_block_boundary");
}

#[test]
fn one_row_strategy_groups_cells_without_losing_provenance() {
    let document = test_document(vec![
        RawBlock {
            id: "row-1-column-1".to_owned(),
            value: RawValue::text("Ada"),
            location: SourceLocation {
                row: Some(1),
                column: Some(1),
                ..SourceLocation::default()
            },
        },
        RawBlock {
            id: "row-1-column-2".to_owned(),
            value: RawValue::text("ada@example.test"),
            location: SourceLocation {
                row: Some(1),
                column: Some(2),
                ..SourceLocation::default()
            },
        },
        RawBlock {
            id: "row-2-column-1".to_owned(),
            value: RawValue::text("Grace"),
            location: SourceLocation {
                row: Some(2),
                column: Some(1),
                ..SourceLocation::default()
            },
        },
    ]);
    let options = SegmentationOptions {
        strategy: SegmentationStrategy::OneRowPerRecord,
        join_separator: " | ".to_owned(),
    };

    let candidates = segment_document(&document, &options);

    assert_eq!(candidates.len(), 2);
    assert_eq!(candidates[0].text, "Ada | ada@example.test");
    assert_eq!(
        candidates[0].source_block_ids,
        vec!["row-1-column-1", "row-1-column-2"]
    );
    assert_eq!(candidates[0].confidence, 0.98);
    assert_eq!(candidates[1].text, "Grace");
}

#[test]
fn indented_continuations_join_with_lower_confidence() {
    let document = test_document(vec![
        RawBlock {
            id: "line-1".to_owned(),
            value: RawValue::text("Name: Ada"),
            location: SourceLocation::default(),
        },
        RawBlock {
            id: "line-2".to_owned(),
            value: RawValue::text("  email: ada@example.test"),
            location: SourceLocation::default(),
        },
        RawBlock {
            id: "line-3".to_owned(),
            value: RawValue::text("Name: Grace"),
            location: SourceLocation::default(),
        },
    ]);
    let options = SegmentationOptions {
        strategy: SegmentationStrategy::JoinIndentedContinuations,
        join_separator: "\n".to_owned(),
    };

    let candidates = segment_document(&document, &options);

    assert_eq!(candidates.len(), 2);
    assert_eq!(candidates[0].text, "Name: Ada\nemail: ada@example.test");
    assert_eq!(candidates[0].confidence, 0.85);
    assert_eq!(candidates[0].reasons[0].code, "indented_continuation");
    assert_eq!(candidates[1].text, "Name: Grace");
}

#[test]
fn heading_boundaries_keep_sections_observable_and_warn_on_indented_followers() {
    let document = test_document(vec![
        RawBlock {
            id: "line-1".to_owned(),
            value: RawValue::text("Name: Ada"),
            location: SourceLocation {
                line: Some(1),
                ..SourceLocation::default()
            },
        },
        RawBlock {
            id: "line-2".to_owned(),
            value: RawValue::text("  email: ada@example.test"),
            location: SourceLocation {
                line: Some(2),
                ..SourceLocation::default()
            },
        },
        RawBlock {
            id: "line-3".to_owned(),
            value: RawValue::text("# Section"),
            location: SourceLocation {
                line: Some(3),
                ..SourceLocation::default()
            },
        },
        RawBlock {
            id: "line-4".to_owned(),
            value: RawValue::text("  section text"),
            location: SourceLocation {
                line: Some(4),
                ..SourceLocation::default()
            },
        },
    ]);
    let options = SegmentationOptions {
        strategy: SegmentationStrategy::JoinIndentedContinuations,
        join_separator: "\n".to_owned(),
    };

    let candidates = segment_document(&document, &options);

    assert_eq!(candidates.len(), 3);
    assert_eq!(candidates[0].text, "Name: Ada\nemail: ada@example.test");
    assert_eq!(candidates[1].text, "# Section");
    assert_eq!(candidates[1].reasons[0].code, "heading_boundary");
    assert_eq!(candidates[2].text, "section text");
    assert_eq!(candidates[2].confidence, 0.35);
    assert_eq!(
        candidates[2].warnings[0].code,
        "ambiguous_heading_continuation"
    );
    assert_eq!(
        candidates[2].warnings[0]
            .location
            .as_ref()
            .and_then(|location| location.line),
        Some(4)
    );
}

#[test]
fn indented_heading_does_not_join_the_previous_record() {
    let document = test_document(vec![
        RawBlock {
            id: "line-1".to_owned(),
            value: RawValue::text("Name: Ada"),
            location: SourceLocation::default(),
        },
        RawBlock {
            id: "line-2".to_owned(),
            value: RawValue::text("  # Section"),
            location: SourceLocation::default(),
        },
    ]);
    let options = SegmentationOptions {
        strategy: SegmentationStrategy::JoinIndentedContinuations,
        join_separator: "\n".to_owned(),
    };

    let candidates = segment_document(&document, &options);

    assert_eq!(candidates.len(), 2);
    assert_eq!(candidates[1].text, "# Section");
    assert_eq!(candidates[1].reasons[0].code, "heading_boundary");
}

#[test]
fn repeated_identifier_strategy_splits_one_block_without_losing_source_reference() {
    let document = test_document(vec![RawBlock {
        id: "line-1".to_owned(),
        value: RawValue::text("ID: first ID: second"),
        location: SourceLocation {
            line: Some(1),
            ..SourceLocation::default()
        },
    }]);
    let options = SegmentationOptions {
        strategy: SegmentationStrategy::SplitRepeatedIdentifiers,
        join_separator: " | ".to_owned(),
    };

    let candidates = segment_document(&document, &options);

    assert_eq!(candidates.len(), 2);
    assert_eq!(candidates[0].text, "ID: first");
    assert_eq!(candidates[1].text, "ID: second");
    assert_eq!(candidates[0].source_block_ids, vec!["line-1"]);
    assert_eq!(candidates[1].source_block_ids, vec!["line-1"]);
    assert_eq!(candidates[0].confidence, 0.82);
    assert_eq!(
        candidates[0].reasons[0].code,
        "repeated_identifier_boundary"
    );
}

#[test]
fn repeated_identifier_strategy_keeps_near_miss_intact() {
    let document = test_document(vec![RawBlock {
        id: "line-1".to_owned(),
        value: RawValue::text("ID: first identifier: second"),
        location: SourceLocation::default(),
    }]);
    let options = SegmentationOptions {
        strategy: SegmentationStrategy::SplitRepeatedIdentifiers,
        join_separator: "\n".to_owned(),
    };

    let candidates = segment_document(&document, &options);

    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].text, "ID: first identifier: second");
    assert_eq!(candidates[0].confidence, 0.9);
    assert!(candidates[0].warnings.is_empty());
}

#[test]
fn repeated_identifier_strategy_reports_ambiguous_marker_sets() {
    let document = test_document(vec![RawBlock {
        id: "line-1".to_owned(),
        value: RawValue::text("ID: first ID: second Record: third Record: fourth"),
        location: SourceLocation {
            line: Some(7),
            ..SourceLocation::default()
        },
    }]);
    let options = SegmentationOptions {
        strategy: SegmentationStrategy::SplitRepeatedIdentifiers,
        join_separator: "\n".to_owned(),
    };

    let candidates = segment_document(&document, &options);

    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].confidence, 0.35);
    assert_eq!(
        candidates[0].reasons[0].code,
        "ambiguous_repeated_identifier_boundary"
    );
    assert_eq!(
        candidates[0].warnings[0]
            .location
            .as_ref()
            .and_then(|location| location.line),
        Some(7)
    );
}

#[test]
fn repeated_identifier_markers_can_be_supplied_by_the_caller() {
    let document = test_document(vec![RawBlock {
        id: "line-1".to_owned(),
        value: RawValue::text("Ref: first Ref: second"),
        location: SourceLocation::default(),
    }]);
    let markers = vec!["Ref:".to_owned()];

    let candidates = segment_document_with_repeated_identifier_markers(&document, &markers);

    assert_eq!(
        candidates
            .iter()
            .map(|candidate| candidate.text.as_str())
            .collect::<Vec<_>>(),
        vec!["Ref: first", "Ref: second"]
    );
}

#[test]
fn segmentation_serializes_candidates_and_keeps_blank_blocks() {
    let document = test_document(vec![RawBlock {
        id: "blank".to_owned(),
        value: RawValue::text(""),
        location: SourceLocation::default(),
    }]);

    let candidates = segment_document(&document, &SegmentationOptions::default());
    let json = serde_json::to_string(&candidates[0]).expect("candidate should serialize");
    let decoded: RecordCandidate =
        serde_json::from_str(&json).expect("candidate should deserialize");

    assert_eq!(decoded, candidates[0]);
    assert_eq!(decoded.source_block_ids, vec!["blank"]);
    assert_eq!(decoded.text, "");
}

#[test]
fn io_error_kind_is_serializable() {
    let error = ParserError::Io {
        path: "missing.txt".to_owned(),
        kind: IoErrorKind::NotFound,
    };

    let json = serde_json::to_string(&error).expect("error should serialize");
    assert_eq!(
        json,
        r#"{"code":"io_error","path":"missing.txt","kind":"not_found"}"#
    );
}

#[test]
fn empty_core_test() {
    assert!(core_ready());
}

fn table_block(sheet: Option<&str>, row: usize, column: usize, value: RawValue) -> RawBlock {
    RawBlock {
        id: format!("block-{row}-{column}"),
        value,
        location: SourceLocation {
            row: Some(row),
            column: Some(column),
            sheet: sheet.map(str::to_owned),
            ..SourceLocation::default()
        },
    }
}

#[allow(clippy::too_many_arguments)]
fn field_of(
    name: &str,
    aliases: &[&str],
    candidate_type: CandidateType,
    required: bool,
    multiple: bool,
) -> AssignmentField {
    AssignmentField {
        name: name.to_owned(),
        aliases: aliases.iter().map(|alias| alias.to_string()).collect(),
        candidate_type,
        required,
        multiple,
        unique: false,
        constraints: Vec::new(),
        expected_column: None,
    }
}

fn table_document(blocks: Vec<RawBlock>) -> RawDocument {
    RawDocument::new(
        "table-document",
        SourceMetadata {
            source_type: SourceType::Csv,
            file_name: Some("table.csv".to_owned()),
            mime_type: Some("text/csv".to_owned()),
            size_bytes: None,
            delimiter: Some(",".to_owned()),
        },
        blocks,
    )
}

#[test]
fn table_headers_are_detected_and_drive_row_assignment() {
    let document = table_document(vec![
        table_block(None, 1, 1, RawValue::text("Email")),
        table_block(None, 1, 2, RawValue::text("Age")),
        table_block(None, 2, 1, RawValue::text("ada@example.test")),
        table_block(None, 2, 2, RawValue::Integer(30)),
    ]);
    let fields = [
        field_of("email", &[], CandidateType::Email, true, false),
        field_of("age", &[], CandidateType::Integer, false, false),
    ];

    let result = parse_document_rows_with_assignment(&document, &fields, &[]);

    assert_eq!(result.sheets.len(), 1);
    let sheet = &result.sheets[0];
    let HeaderExtraction::Detected { headers } = &sheet.header else {
        panic!("expected a detected header");
    };
    assert_eq!(headers.source_row, 1);
    assert_eq!(
        headers.labels,
        vec![(1, "Email".to_owned()), (2, "Age".to_owned())]
    );
    assert_eq!(sheet.records.len(), 1);
    let record = &sheet.records[0];
    assert_eq!(record.source_row, 2);
    assert_eq!(record.parse.assignment.fields.len(), 2);
    assert_eq!(
        record.parse.assignment.fields[0].candidates[0].raw_value,
        "ada@example.test"
    );
    assert_eq!(
        record.parse.assignment.fields[0].candidates[0].source_column,
        Some(1)
    );
    assert_eq!(
        record.parse.assignment.fields[1].candidates[0].normalized_value,
        Some(serde_json::json!(30))
    );
}

#[test]
fn typed_first_rows_are_not_headers_and_stay_records() {
    let document = table_document(vec![
        table_block(None, 1, 1, RawValue::text("2024-01-15")),
        table_block(None, 1, 2, RawValue::text("Note")),
        table_block(None, 2, 1, RawValue::text("Contact")),
        table_block(None, 2, 2, RawValue::text("ada@example.test")),
    ]);
    let fields = [field_of("email", &[], CandidateType::Email, false, false)];

    let result = parse_document_rows_with_assignment(&document, &fields, &[]);

    let sheet = &result.sheets[0];
    let HeaderExtraction::NotDetected { code, .. } = &sheet.header else {
        panic!("expected header detection to be rejected");
    };
    assert_eq!(code, "header_not_detected_strong_values");
    assert_eq!(sheet.records.len(), 2);
    // Without a header the email still assigns by type compatibility.
    assert_eq!(
        sheet.records[1].parse.assignment.fields[0].candidates[0].raw_value,
        "ada@example.test"
    );
}

#[test]
fn single_row_documents_do_not_invent_a_header() {
    let document = table_document(vec![
        table_block(None, 1, 1, RawValue::text("Email")),
        table_block(None, 1, 2, RawValue::text("Age")),
    ]);

    let result = parse_document_rows_with_assignment(&document, &[], &[]);

    let sheet = &result.sheets[0];
    let HeaderExtraction::NotDetected { code, message } = &sheet.header else {
        panic!("expected header detection to be rejected");
    };
    assert_eq!(code, "header_not_detected_single_row");
    assert!(!message.is_empty());
    assert_eq!(sheet.records.len(), 1);
}

#[test]
fn header_labels_resolve_ambiguous_integer_columns() {
    let document = table_document(vec![
        table_block(None, 1, 1, RawValue::text("Quantity")),
        table_block(None, 1, 2, RawValue::text("Age")),
        table_block(None, 2, 1, RawValue::Integer(30)),
        table_block(None, 2, 2, RawValue::Integer(7)),
    ]);
    let fields = [field_of("age", &[], CandidateType::Integer, true, false)];

    let result = parse_document_rows_with_assignment(&document, &fields, &[]);

    let record = &result.sheets[0].records[0];
    let assignment = &record.parse.assignment;
    // Both integers are type-compatible; only the "Age" column matches.
    assert_eq!(assignment.warnings.len(), 0);
    assert_eq!(assignment.fields[0].candidates.len(), 1);
    let selected = &assignment.fields[0].candidates[0];
    assert_eq!(selected.source_column, Some(2));
    assert_eq!(selected.normalized_value, Some(serde_json::json!(7)));
    assert!(
        selected
            .reasons
            .iter()
            .any(|reason| reason.code == "header_label_match")
    );
    assert_eq!(assignment.unassigned_candidates.len(), 1);
}

#[test]
fn multiple_fields_prefer_header_matching_columns() {
    let document = table_document(vec![
        table_block(None, 1, 1, RawValue::text("Phone")),
        table_block(None, 1, 2, RawValue::text("Fax")),
        table_block(None, 2, 1, RawValue::text("+15550101")),
        table_block(None, 2, 2, RawValue::text("+15550102")),
    ]);
    let fields = [field_of(
        "phone",
        &[],
        CandidateType::PhoneNumber,
        false,
        true,
    )];

    let result = parse_document_rows_with_assignment(&document, &fields, &[]);

    let assignment = &result.sheets[0].records[0].parse.assignment;
    assert_eq!(assignment.fields[0].candidates.len(), 1);
    assert_eq!(assignment.fields[0].candidates[0].raw_value, "+15550101");
    // Each phone-like token also surfaces an integer candidate (evidence,
    // not assignments); together with the unmatched "Fax" phone they stay
    // observable as unassigned.
    let unassigned_phones = assignment
        .unassigned_candidates
        .iter()
        .filter(|candidate| candidate.candidate_type == CandidateType::PhoneNumber)
        .count();
    assert_eq!(assignment.unassigned_candidates.len(), 3);
    assert_eq!(unassigned_phones, 1);
    assert_eq!(assignment.unassigned_candidates[0].source_column, Some(1));
}

#[test]
fn blocks_without_row_provenance_are_reported_not_dropped() {
    let document = table_document(vec![
        table_block(None, 1, 1, RawValue::text("Email")),
        table_block(None, 1, 2, RawValue::text("Age")),
        table_block(None, 2, 1, RawValue::text("ada@example.test")),
        table_block(None, 2, 2, RawValue::Integer(30)),
        RawBlock {
            id: "loose-block".to_owned(),
            value: RawValue::text("stray note"),
            location: SourceLocation::default(),
        },
    ]);

    let grouped = group_document_rows(&document);

    assert_eq!(grouped.rows.len(), 2);
    assert_eq!(grouped.warnings.len(), 1);
    assert_eq!(grouped.warnings[0].code, "row_provenance_missing");
    assert_eq!(
        grouped.warnings[0].location.as_ref().and_then(|l| l.row),
        None
    );
}

#[test]
fn sheets_get_independent_header_contexts() {
    let document = table_document(vec![
        table_block(Some("A"), 1, 1, RawValue::text("Email")),
        table_block(Some("A"), 1, 2, RawValue::text("Age")),
        table_block(Some("A"), 2, 1, RawValue::text("ada@example.test")),
        table_block(Some("A"), 2, 2, RawValue::Integer(30)),
        table_block(Some("B"), 1, 1, RawValue::text("Phone")),
        table_block(Some("B"), 1, 2, RawValue::text("Fax")),
        table_block(Some("B"), 2, 1, RawValue::text("+15550101")),
        table_block(Some("B"), 2, 2, RawValue::text("+15550102")),
    ]);
    let fields = [
        field_of("email", &[], CandidateType::Email, false, false),
        field_of("phone", &[], CandidateType::PhoneNumber, false, false),
    ];

    let result = parse_document_rows_with_assignment(&document, &fields, &[]);

    assert_eq!(result.sheets.len(), 2);
    assert_eq!(result.sheets[0].sheet.as_deref(), Some("A"));
    assert_eq!(result.sheets[1].sheet.as_deref(), Some("B"));
    for sheet in &result.sheets {
        assert!(sheet.header.context().is_some());
        assert_eq!(sheet.records.len(), 1);
    }
    let assigned_value = |sheet: &SheetTableResult, name: &str| {
        sheet.records[0]
            .parse
            .assignment
            .fields
            .iter()
            .find(|field| field.name == name)
            .map(|field| field.candidates[0].raw_value.clone())
    };
    // Sheets only report fields that found a compatible candidate.
    assert_eq!(
        assigned_value(&result.sheets[0], "email").as_deref(),
        Some("ada@example.test")
    );
    assert_eq!(assigned_value(&result.sheets[0], "phone"), None);
    assert_eq!(
        assigned_value(&result.sheets[1], "phone").as_deref(),
        Some("+15550101")
    );
    assert_eq!(assigned_value(&result.sheets[1], "email"), None);
}

#[test]
fn table_parse_result_round_trips_as_json() {
    let document = table_document(vec![
        table_block(None, 1, 1, RawValue::text("Email")),
        table_block(None, 1, 2, RawValue::text("Age")),
        table_block(None, 2, 1, RawValue::text("ada@example.test")),
        table_block(None, 2, 2, RawValue::Integer(30)),
    ]);
    let fields = [field_of("email", &[], CandidateType::Email, true, false)];

    let result = parse_document_rows_with_assignment(&document, &fields, &[]);
    let json = serde_json::to_string(&result).expect("table result should serialize");
    let decoded: TableParseResult =
        serde_json::from_str(&json).expect("table result should deserialize");

    assert_eq!(decoded, result);
}

#[test]
fn parse_document_dispatches_to_table_for_row_provenance() {
    let document = table_document(vec![
        table_block(None, 1, 1, RawValue::text("Email")),
        table_block(None, 1, 2, RawValue::text("Age")),
        table_block(None, 2, 1, RawValue::text("ada@example.test")),
        table_block(None, 2, 2, RawValue::Integer(30)),
    ]);
    let fields = [field_of("email", &[], CandidateType::Email, true, false)];

    let response =
        parse_document_with_assignment(&document, &fields, &[], Some("contact".to_owned()));

    assert_eq!(response.contract_version, CONTRACT_VERSION);
    assert_eq!(response.parser_version, env!("CARGO_PKG_VERSION"));
    assert_eq!(response.record_name.as_deref(), Some("contact"));
    assert_eq!(response.source_type, SourceType::Csv);
    let ParseContent::Table { sheets } = &response.content else {
        panic!("expected table content");
    };
    assert_eq!(sheets.len(), 1);
    assert_eq!(sheets[0].records.len(), 1);
    assert_eq!(
        sheets[0].records[0].parse.assignment.fields[0].candidates[0].raw_value,
        "ada@example.test"
    );
}

#[test]
fn parse_document_dispatches_to_text_without_row_provenance() {
    let document = test_document(vec![
        RawBlock {
            id: "b1".to_owned(),
            value: RawValue::text("ada@example.test"),
            location: SourceLocation::default(),
        },
        RawBlock {
            id: "b2".to_owned(),
            value: RawValue::text("grace@example.test"),
            location: SourceLocation::default(),
        },
    ]);
    let fields = [field_of("email", &[], CandidateType::Email, true, false)];

    let response = parse_document_with_assignment(&document, &fields, &[], None);

    assert_eq!(response.source_type, SourceType::Text);
    assert!(response.warnings.is_empty());
    let ParseContent::Text { records } = &response.content else {
        panic!("expected text content");
    };
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].source_block_id, "b1");
    assert_eq!(
        records[0].parse.assignment.fields[0].candidates[0].raw_value,
        "ada@example.test"
    );
    assert_eq!(
        records[1].parse.assignment.fields[0].candidates[0].raw_value,
        "grace@example.test"
    );
}

#[test]
fn parse_response_round_trips_as_json() {
    let document = table_document(vec![
        table_block(None, 1, 1, RawValue::text("Email")),
        table_block(None, 1, 2, RawValue::text("Age")),
        table_block(None, 2, 1, RawValue::text("ada@example.test")),
        table_block(None, 2, 2, RawValue::Integer(30)),
    ]);
    let fields = [field_of("email", &[], CandidateType::Email, true, false)];

    let response =
        parse_document_with_assignment(&document, &fields, &[], Some("contact".to_owned()));
    let json = serde_json::to_string(&response).expect("response should serialize");
    let decoded: ParseResponse = serde_json::from_str(&json).expect("response should deserialize");

    assert_eq!(decoded, response);
}

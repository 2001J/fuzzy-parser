use super::*;

#[test]
fn standalone_and_supplied_candidates_share_contextual_selection_without_trusting_reasons() {
    let fields = [field_of(
        "name",
        &["person"],
        CandidateType::PersonName,
        true,
        false,
    )];
    let text = "person: Zoë  東京";
    let parsed = parse_text_with_assignment(text, &fields, &[]);
    assert_eq!(
        parsed.assignment.fields[0].candidates[0].raw_value,
        "Zoë  東京"
    );
    assert_eq!(
        assign_candidates(text, &parsed.candidates, &fields),
        parsed.assignment
    );
    assert!(
        parsed
            .candidates
            .iter()
            .all(|c| c.source_reference.is_none())
    );

    let text = "Ada Lovelace";
    let mut parsed = parse_text_with_assignment(text, &fields, &[]);
    for candidate in &mut parsed.candidates {
        candidate.confidence = 1.0;
        candidate.reasons = vec![Reason::new(
            "caller_label_match",
            "untrusted supplied reason",
        )];
    }
    let result = assign_candidates(text, &parsed.candidates, &fields);
    assert!(result.fields.is_empty());
    assert_eq!(result.unassigned_candidates, parsed.candidates);
    assert_eq!(result.warnings[0].code, "required_field_missing");
}

#[test]
fn supplied_single_type_cannot_hide_competing_text_name_ownership() {
    for header in [false, true] {
        let text = if header {
            "Ada Lovelace"
        } else {
            "value: Ada Lovelace"
        };
        let fields = [
            field_of("value", &[], CandidateType::Text, true, false),
            field_of("person", &["value"], CandidateType::PersonName, true, false),
        ];
        let mut candidates =
            parse_text_with_assignment("value: Ada Lovelace", &fields[..1], &[]).candidates;
        if header {
            candidates[0].source_span = TextSpan {
                byte_start: 0,
                byte_end: text.len(),
            };
            candidates[0].source_column = Some(1);
        }
        let context = TableHeaderContext {
            sheet: None,
            source_row: 1,
            labels: vec![(1, "value".into())],
            source_block_ids: vec![],
        };
        for fields in [fields.to_vec(), fields.into_iter().rev().collect()] {
            let result = assign_candidates_with_header_context(
                text,
                &candidates,
                &fields,
                header.then_some(&context),
            );
            assert!(result.fields.is_empty());
            assert_eq!(result.unassigned_candidates, candidates);
            assert!(
                result
                    .warnings
                    .iter()
                    .any(|w| w.code == "text_field_ambiguous")
            );
        }
    }
}

#[test]
fn supplied_text_overlap_uses_intervals_block_indices_and_coordinate_spaces() {
    let text = "note: Ada ada@example.test";
    let fields = [
        field_of("note", &[], CandidateType::Text, true, false),
        field_of("email", &[], CandidateType::Email, true, false),
    ];
    let mut candidates = parse_text_with_assignment(text, &fields[..1], &[]).candidates;
    assert_eq!(candidates.len(), 2);
    for candidate in &mut candidates {
        candidate.source_reference = Some(SourceReference {
            block_index: 0,
            coordinate_space: SourceCoordinateSpace::RawTextUtf8,
            span: candidate.source_span.clone(),
        });
    }
    for (block, space, overlap) in [
        (0, SourceCoordinateSpace::RawTextUtf8, true),
        (1, SourceCoordinateSpace::RawTextUtf8, false),
        (0, SourceCoordinateSpace::RenderedValueUtf8, false),
    ] {
        let mut candidates = candidates.clone();
        let reference = candidates
            .iter_mut()
            .find(|c| c.candidate_type == CandidateType::Text)
            .unwrap()
            .source_reference
            .as_mut()
            .unwrap();
        reference.block_index = block;
        reference.coordinate_space = space;
        let result = assign_candidates(text, &candidates, &fields);
        assert_eq!(result.fields.iter().any(|f| f.name == "note"), !overlap);
        assert!(result.fields.iter().any(|f| f.name == "email"));
        assert_eq!(
            result
                .warnings
                .iter()
                .any(|w| w.code == "text_evidence_overlap"),
            overlap
        );
    }
}

#[test]
fn overlapping_new_supplied_intervals_abstain_before_field_order_selection() {
    let text = "first: Alpha; second: Beta";
    let fields = [
        field_of("first", &[], CandidateType::Text, true, false),
        field_of("second", &[], CandidateType::PersonName, true, false),
    ];
    let mut candidates = parse_text_with_assignment(text, &fields, &[]).candidates;
    for candidate in &mut candidates {
        candidate.source_reference = Some(SourceReference {
            block_index: 0,
            coordinate_space: SourceCoordinateSpace::RawTextUtf8,
            span: TextSpan {
                byte_start: 0,
                byte_end: candidate.raw_value.len(),
            },
        });
    }
    for fields in [fields.to_vec(), fields.into_iter().rev().collect()] {
        let result = assign_candidates(text, &candidates, &fields);
        assert!(result.fields.is_empty());
        assert_eq!(result.unassigned_candidates, candidates);
        assert!(
            result
                .warnings
                .iter()
                .any(|w| w.code == "text_field_ambiguous")
        );
    }
}

#[test]
fn new_candidate_tags_roundtrip_without_changing_contract_versions() {
    for (kind, tag) in [
        (CandidateType::Text, "text"),
        (CandidateType::PersonName, "person_name"),
    ] {
        assert_eq!(serde_json::to_value(&kind).unwrap(), tag);
        assert_eq!(
            serde_json::from_value::<CandidateType>(serde_json::json!(tag)).unwrap(),
            kind
        );
        let fields = [field_of("name", &[], kind, true, false)];
        let response = parse_document_with_assignment(
            &test_document(vec![RawBlock {
                id: "same".into(),
                value: RawValue::text("name: Ada Lovelace"),
                location: SourceLocation::default(),
            }]),
            &fields,
            &[],
            None,
        );
        let value = serde_json::to_value(&response).unwrap();
        assert_eq!(value["contract_version"], "0.1");
        let mut reasons = serde_json::json!([{"code":"caller_label_match","message":"the value follows a literal caller-provided field label"}]);
        if tag == "person_name" {
            reasons.as_array_mut().unwrap().push(serde_json::json!({"code":"caller_person_name","message":"the caller requests a possible person name; this is not identity verification"}));
        }
        assert_eq!(
            value["content"]["records"][0]["parse"]["candidates"][0],
            serde_json::json!({
                "candidate_type":tag,"raw_value":"Ada Lovelace","normalized_value":"Ada Lovelace",
                "source_span":{"byte_start":6,"byte_end":18},"source_column":null,
                "source_reference":{"block_index":0,"coordinate_space":"raw_text_utf8","span":{"byte_start":6,"byte_end":18}},
                "confidence":0.8,"reasons":reasons
            })
        );
        assert_eq!(
            serde_json::from_value::<ParseResponse>(value).unwrap(),
            response
        );
        assert_complete_source_evidence(&response);
    }
}

#[test]
fn legacy_unique_warning_remains_available_for_multiple_new_type_values() {
    let mut field = field_of("name", &[], CandidateType::PersonName, true, true);
    field.unique = true;
    let result = parse_text_with_assignment("name: Ada; name: Grace", &[field], &[]);
    assert_eq!(result.assignment.fields[0].candidates.len(), 2);
    assert_eq!(
        result.assignment.warnings[0].code,
        "unique_field_multiple_values"
    );
}

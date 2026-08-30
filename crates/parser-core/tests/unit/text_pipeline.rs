use super::*;

fn document(values: &[(&str, &str)]) -> RawDocument {
    RawDocument::new(
        "pipeline",
        SourceMetadata {
            source_type: SourceType::Text,
            file_name: None,
            mime_type: None,
            size_bytes: None,
            delimiter: None,
        },
        values
            .iter()
            .map(|(id, value)| RawBlock {
                id: (*id).to_owned(),
                value: RawValue::text(*value),
                location: SourceLocation::default(),
            })
            .collect(),
    )
}

fn field(name: &str, kind: CandidateType, required: bool, multiple: bool) -> PlanField {
    PlanField::new(
        AssignmentField {
            name: name.to_owned(),
            aliases: Vec::new(),
            candidate_type: kind,
            required,
            multiple,
            unique: false,
            constraints: Vec::new(),
            expected_column: None,
        },
        Vec::new(),
    )
}

fn options(strategy: TextPipelineStrategy, markers: &[&str]) -> TextPipelineOptions {
    TextPipelineOptions {
        normalization: NormalizationOptions::default(),
        strategy,
        repeated_identifier_markers: markers.iter().map(|value| (*value).to_owned()).collect(),
    }
}

fn parse(
    document: &RawDocument,
    fields: Vec<PlanField>,
    options: TextPipelineOptions,
) -> ParseResponse {
    parse_document_with_plan(
        document,
        &ParsePlan::new(fields, Some("synthetic".to_owned())).with_text_pipeline(options),
    )
}

fn records(response: &ParseResponse) -> &[TextRecordParseResult] {
    match &response.content {
        ParseContent::Text { records } => records,
        ParseContent::Table { .. } => panic!("expected text mode"),
    }
}

#[test]
fn mapped_normalization_partitions_raw_and_composed_utf8_coordinates() {
    let input = " \r\n\u{2014}\t  email: ADA@example.test  ";
    let document = document(&[("same", input)]);
    let response = parse(
        &document,
        vec![field("email", CandidateType::Email, true, false)],
        options(TextPipelineStrategy::OneBlockPerRecord, &[]),
    );
    let record = &records(&response)[0];
    let composition = record.composition.as_ref().unwrap();
    assert_eq!(composition.composed_text, "- email: ADA@example.test");
    assert!(crate::text_pipeline::validate_composition(
        &document,
        composition
    ));
    let operations: Vec<_> = composition
        .segments
        .iter()
        .filter_map(|segment| match segment {
            TextCompositionSegment::Source { mapping_runs, .. } => Some(mapping_runs),
            TextCompositionSegment::SyntheticSeparator { .. } => None,
        })
        .flatten()
        .flat_map(|run| &run.operations)
        .copied()
        .collect();
    for expected in [
        TextMappingOperation::Unchanged,
        TextMappingOperation::LineEndingFold,
        TextMappingOperation::PunctuationReplacement,
        TextMappingOperation::Trim,
        TextMappingOperation::WhitespaceCollapse,
    ] {
        assert!(operations.contains(&expected), "missing {expected:?}");
    }
    let candidate = &record.parse.assignment.fields[0].candidates[0];
    assert_eq!(candidate.raw_value, "ADA@example.test");
    assert_eq!(
        candidate
            .source_reference
            .as_ref()
            .unwrap()
            .resolve(&document)
            .as_deref(),
        Some("ADA@example.test")
    );
    assert_eq!(
        response.source_evidence.as_ref().unwrap().document,
        document
    );
}

#[test]
fn joined_segments_insert_only_synthetic_newlines_and_keep_candidates_local() {
    let document = document(&[
        ("same", "email: ada@example.test"),
        ("same", "  enabled: yes"),
    ]);
    let response = parse(
        &document,
        vec![
            field("email", CandidateType::Email, true, false),
            field("enabled", CandidateType::Boolean, true, false),
        ],
        options(TextPipelineStrategy::JoinIndentedContinuations, &[]),
    );
    let record = &records(&response)[0];
    let composition = record.composition.as_ref().unwrap();
    assert_eq!(record.source_block_id, "same");
    assert_eq!(
        composition.composed_text,
        "email: ada@example.test\nenabled: yes"
    );
    assert!(crate::text_pipeline::validate_composition(
        &document,
        composition
    ));
    let source_spans: Vec<_> = composition
        .segments
        .iter()
        .filter_map(|segment| match segment {
            TextCompositionSegment::Source { composed_span, .. } => Some(composed_span),
            TextCompositionSegment::SyntheticSeparator { composed_span } => {
                assert_eq!(
                    &composition.composed_text[composed_span.byte_start..composed_span.byte_end],
                    "\n"
                );
                None
            }
        })
        .collect();
    assert_eq!(source_spans.len(), 2);
    for candidate in &record.parse.candidates {
        assert!(source_spans.iter().any(|span| {
            span.byte_start <= candidate.source_span.byte_start
                && candidate.source_span.byte_end <= span.byte_end
        }));
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
    assert_eq!(record.parse.assignment.fields.len(), 2);
}

#[test]
fn labels_and_singular_ties_never_cross_joined_segment_boundaries() {
    let document = document(&[
        ("label", "prefix email:"),
        ("value", "  ada@example.test"),
        ("value", "  grace@example.test"),
    ]);
    let response = parse(
        &document,
        vec![field("email", CandidateType::Email, true, false)],
        options(TextPipelineStrategy::JoinIndentedContinuations, &[]),
    );
    let record = &records(&response)[0];
    assert!(record.parse.assignment.fields.is_empty());
    assert_eq!(record.parse.assignment.unassigned_candidates.len(), 2);
    assert_eq!(
        record
            .parse
            .assignment
            .warnings
            .iter()
            .map(|warning| warning.code.as_str())
            .collect::<Vec<_>>(),
        ["multiple_candidates_ambiguous", "required_field_missing"]
    );

    let text_response = parse(
        &document,
        vec![
            field("email", CandidateType::Email, false, true),
            field("prefix email", CandidateType::Text, false, false),
        ],
        options(TextPipelineStrategy::JoinIndentedContinuations, &[]),
    );
    let text_record = &records(&text_response)[0];
    assert!(
        text_record
            .parse
            .assignment
            .fields
            .iter()
            .all(|field| field.name != "prefix email")
    );
}

#[test]
fn repeated_identifier_splits_and_duplicate_block_ids_use_authoritative_indexes() {
    let document = document(&[
        ("duplicate", "entry: 1 entry: 2"),
        ("duplicate", "entry: 3 entry: 4"),
    ]);
    let response = parse(
        &document,
        vec![field("number", CandidateType::Integer, false, false)],
        options(TextPipelineStrategy::SplitRepeatedIdentifiers, &["entry:"]),
    );
    let records = records(&response);
    assert_eq!(records.len(), 4);
    assert_eq!(
        records
            .iter()
            .map(|record| record.source_block_id.as_str())
            .collect::<Vec<_>>(),
        ["duplicate", "duplicate", "duplicate", "duplicate"]
    );
    assert_eq!(
        records
            .iter()
            .map(
                |record| match &record.composition.as_ref().unwrap().segments[0] {
                    TextCompositionSegment::Source {
                        source_reference, ..
                    } => source_reference.block_index,
                    TextCompositionSegment::SyntheticSeparator { .. } => unreachable!(),
                }
            )
            .collect::<Vec<_>>(),
        [0, 0, 1, 1]
    );
    for record in records {
        let composition = record.composition.as_ref().unwrap();
        assert!(crate::text_pipeline::validate_composition(
            &document,
            composition
        ));
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
fn heading_boundary_warning_stays_in_composition_and_adds_one_review_reason() {
    let document = document(&[
        ("heading", "# Section"),
        ("value", "  email: ada@example.test"),
    ]);
    let response = parse(
        &document,
        vec![field("email", CandidateType::Email, false, false)],
        options(TextPipelineStrategy::JoinIndentedContinuations, &[]),
    );
    assert!(response.warnings.is_empty());
    let records = records(&response);
    assert_eq!(records.len(), 2);
    let second = &records[1];
    assert_eq!(
        second.composition.as_ref().unwrap().boundary.warnings[0].code,
        "ambiguous_heading_continuation"
    );
    assert_eq!(
        second
            .parse
            .review
            .as_ref()
            .unwrap()
            .reasons
            .iter()
            .filter(|reason| reason.code == "composition_boundary_warnings")
            .count(),
        1
    );
}

#[test]
fn blank_source_membership_keeps_zero_length_composed_and_complete_raw_mapping() {
    let document = document(&[("blank", " \t  ")]);
    let response = parse(
        &document,
        Vec::new(),
        options(TextPipelineStrategy::OneBlockPerRecord, &[]),
    );
    let record = &records(&response)[0];
    let composition = record.composition.as_ref().unwrap();
    assert!(composition.composed_text.is_empty());
    assert!(crate::text_pipeline::validate_composition(
        &document,
        composition
    ));
    let TextCompositionSegment::Source {
        source_reference,
        composed_span,
        mapping_runs,
    } = &composition.segments[0]
    else {
        panic!("blank record remains a source segment");
    };
    assert_eq!(
        source_reference.span,
        TextSpan {
            byte_start: 0,
            byte_end: 4
        }
    );
    assert_eq!(
        composed_span,
        &TextSpan {
            byte_start: 0,
            byte_end: 0
        }
    );
    assert!(mapping_runs.iter().all(|run| {
        run.composed_span
            == TextSpan {
                byte_start: 0,
                byte_end: 0,
            }
            && run.operations.contains(&TextMappingOperation::Trim)
    }));
    assert_eq!(
        response.source_evidence.as_ref().unwrap().blocks[0].unused_spans,
        [TextSpan {
            byte_start: 0,
            byte_end: 4
        }]
    );
}

#[test]
fn text_multiple_and_length_constraints_use_original_values_in_source_order() {
    let document = document(&[
        ("first", "note: Zoë  東京"),
        ("second", "  note: Call later"),
    ]);
    let note = PlanField::new(
        AssignmentField {
            name: "note".to_owned(),
            aliases: Vec::new(),
            candidate_type: CandidateType::Text,
            required: true,
            multiple: true,
            unique: false,
            constraints: vec![
                AssignmentConstraint::MinimumLength(7),
                AssignmentConstraint::MaximumLength(10),
            ],
            expected_column: None,
        },
        Vec::new(),
    );
    let response = parse(
        &document,
        vec![note],
        options(TextPipelineStrategy::JoinIndentedContinuations, &[]),
    );
    let record = &records(&response)[0];
    let assigned = &record.parse.assignment.fields[0].candidates;
    assert_eq!(
        assigned
            .iter()
            .map(|candidate| candidate.raw_value.as_str())
            .collect::<Vec<_>>(),
        ["Zoë  東京", "Call later"]
    );
    assert!(assigned.iter().all(|candidate| {
        candidate
            .normalized_value
            .as_ref()
            .and_then(serde_json::Value::as_str)
            == Some(candidate.raw_value.as_str())
            && candidate
                .source_reference
                .as_ref()
                .unwrap()
                .resolve(&document)
                .as_deref()
                == Some(candidate.raw_value.as_str())
    }));
}

#[test]
fn competing_repeated_markers_keep_one_record_and_boundary_review_evidence() {
    let document = document(&[("record", "id: 1 key: a id: 2 key: b")]);
    let response = parse(
        &document,
        Vec::new(),
        options(
            TextPipelineStrategy::SplitRepeatedIdentifiers,
            &["id:", "key:"],
        ),
    );
    let records = records(&response);
    assert_eq!(records.len(), 1);
    let record = &records[0];
    let composition = record.composition.as_ref().unwrap();
    assert_eq!(composition.composed_text, "id: 1 key: a id: 2 key: b");
    assert_eq!(
        composition.boundary.warnings[0].code,
        "ambiguous_repeated_identifier_boundary"
    );
    assert_eq!(
        record
            .parse
            .review
            .as_ref()
            .unwrap()
            .reasons
            .last()
            .unwrap()
            .code,
        "composition_boundary_warnings"
    );
    assert!(response.warnings.is_empty());
}

#[test]
fn disabled_normalization_is_echoed_and_keeps_every_mapping_run_unchanged() {
    let document = document(&[("raw", "  \u{2014}\tvalue  ")]);
    let disabled = NormalizationOptions {
        normalize_line_endings: false,
        trim_whitespace: false,
        collapse_whitespace: false,
        normalize_punctuation: false,
        mark_noise: false,
    };
    let response = parse(
        &document,
        Vec::new(),
        TextPipelineOptions {
            normalization: disabled.clone(),
            strategy: TextPipelineStrategy::OneBlockPerRecord,
            repeated_identifier_markers: Vec::new(),
        },
    );
    let composition = records(&response)[0].composition.as_ref().unwrap();
    assert_eq!(composition.composed_text, "  \u{2014}\tvalue  ");
    assert_eq!(composition.applied_options.normalization, disabled);
    let TextCompositionSegment::Source { mapping_runs, .. } = &composition.segments[0] else {
        panic!("one block has one source segment");
    };
    assert!(mapping_runs.iter().all(|run| {
        run.operations == [TextMappingOperation::Unchanged]
            && run.raw_span.byte_end - run.raw_span.byte_start
                == run.composed_span.byte_end - run.composed_span.byte_start
    }));
}

#[test]
fn composition_addition_round_trips_under_contract_0_1_with_stable_tags() {
    let document = document(&[("record", "entry: 1 entry: 2")]);
    let response = parse(
        &document,
        Vec::new(),
        options(TextPipelineStrategy::SplitRepeatedIdentifiers, &["entry:"]),
    );
    assert_eq!(response.contract_version, CONTRACT_VERSION);
    let wire = serde_json::to_value(&response).unwrap();
    assert_eq!(
        wire["content"]["records"][0]["composition"]["applied_options"]["strategy"],
        "split_repeated_identifiers"
    );
    assert_eq!(
        wire["content"]["records"][0]["composition"]["segments"][0]["kind"],
        "source"
    );
    assert_eq!(
        serde_json::from_value::<ParseResponse>(wire).unwrap(),
        response
    );
    assert!(serde_json::from_str::<TextMappingOperation>("\"future_operation\"").is_err());
}

#[test]
fn table_warning_order_is_input_then_grouping_then_not_applied() {
    let mut document = table_document(vec![
        table_block(None, 1, 1, RawValue::text("Email")),
        table_block(None, 1, 2, RawValue::text("Enabled")),
        table_block(None, 2, 1, RawValue::text("ada@example.test")),
        table_block(None, 2, 2, RawValue::text("yes")),
        RawBlock {
            id: "excluded".to_owned(),
            value: RawValue::text("unused"),
            location: SourceLocation::default(),
        },
    ]);
    document.warnings.push(ParserWarning {
        code: "input_warning".to_owned(),
        message: "synthetic input warning".to_owned(),
        location: None,
    });
    let response = parse(
        &document,
        Vec::new(),
        options(TextPipelineStrategy::OneBlockPerRecord, &[]),
    );
    assert!(matches!(response.content, ParseContent::Table { .. }));
    assert_eq!(
        response
            .warnings
            .iter()
            .map(|warning| warning.code.as_str())
            .collect::<Vec<_>>(),
        [
            "input_warning",
            "row_provenance_missing",
            "text_pipeline_not_applied"
        ]
    );
    assert_eq!(
        response.source_evidence.as_ref().unwrap().document,
        document
    );
}

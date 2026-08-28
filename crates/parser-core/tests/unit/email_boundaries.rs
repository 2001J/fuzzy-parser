use super::*;

fn assert_emails(text: &str, expected: &[(&str, usize, usize)], unused: &[&str]) {
    let document = test_document(vec![RawBlock {
        id: "email-boundaries".to_owned(),
        value: RawValue::text(text),
        location: SourceLocation::default(),
    }]);
    let fields = [field_of("email", &[], CandidateType::Email, true, false)];
    let response = parse_document_with_assignment(&document, &fields, &[], None);
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
    assert_complete_source_evidence(&response);
    let evidence = response.source_evidence.as_ref().unwrap();
    assert_eq!(evidence.document, document);
    let ParseContent::Text { records } = &response.content else {
        panic!("punctuation must not select table mode");
    };
    assert_eq!(records.len(), 1);
    let parse = &records[0].parse;
    assert_eq!(parse.candidates.len(), expected.len(), "{text:?}");
    let standalone = detect_email_candidates(text);
    assert_eq!(standalone.len(), expected.len());
    for ((candidate, standalone), &(raw, start, end)) in
        parse.candidates.iter().zip(&standalone).zip(expected)
    {
        let span = TextSpan {
            byte_start: start,
            byte_end: end,
        };
        assert_eq!(candidate.candidate_type, CandidateType::Email);
        assert_eq!(candidate.raw_value, raw);
        assert_eq!(
            candidate.normalized_value,
            Some(serde_json::json!(raw.to_ascii_lowercase()))
        );
        assert_eq!(candidate.source_span, span);
        assert_eq!(&text[start..end], raw);
        assert_eq!(
            candidate.source_reference,
            Some(SourceReference {
                block_index: 0,
                coordinate_space: SourceCoordinateSpace::RawTextUtf8,
                span,
            })
        );
        let mut without_reference = candidate.clone();
        without_reference.source_reference = None;
        assert_eq!(&without_reference, standalone);
    }
    let codes: Vec<_> = parse
        .assignment
        .warnings
        .iter()
        .map(|warning| warning.code.as_str())
        .collect();
    assert_eq!(
        codes,
        match expected.len() {
            0 => vec!["required_field_missing"],
            1 => vec![],
            _ => vec!["multiple_candidates_ambiguous"],
        }
    );
    if expected.is_empty() {
        assert!(parse.assignment.fields.is_empty());
        assert!(parse.assignment.unassigned_candidates.is_empty());
    } else {
        assert_eq!(parse.assignment.fields.len(), 1);
        assert_eq!(parse.assignment.fields[0].candidates, parse.candidates[..1]);
        assert_eq!(
            parse.assignment.unassigned_candidates,
            parse.candidates[1..]
        );
    }
    let unused_text: Vec<_> = evidence.blocks[0]
        .unused_spans
        .iter()
        .map(|span| &text[span.byte_start..span.byte_end])
        .collect();
    assert_eq!(unused_text, unused);
    assert_eq!(
        parse.review.as_ref().unwrap().status,
        RecordReviewStatus::NeedsReview
    );

    // Without a matching field, every detected copy must survive unassigned.
    let unassigned = parse_document_with_assignment(&document, &[], &[], None);
    assert_complete_source_evidence(&unassigned);
    let unassigned_parse = response_parses(&unassigned)[0];
    assert_eq!(
        unassigned_parse.assignment.unassigned_candidates,
        parse.candidates
    );
}

#[test]
fn comma_adjacent_email_preserves_source() {
    assert_emails(
        "Ada Lovelace,ada@example.test",
        &[("ada@example.test", 13, 29)],
        &["Ada Lovelace,"],
    );
}

#[test]
fn spaced_email_control() {
    assert_emails(
        "Ada Lovelace ada@example.test",
        &[("ada@example.test", 13, 29)],
        &["Ada Lovelace "],
    );
}

#[test]
fn unicode_prefix_uses_original_byte_offsets() {
    assert_emails(
        "Zoë 東京,ADA@example.test",
        &[("ADA@example.test", 12, 28)],
        &["Zoë 東京,"],
    );
}

#[test]
fn multiple_and_repeated_addresses_keep_distinct_spans() {
    assert_emails(
        "ada@example.test,ada@example.test;grace@example.test",
        &[
            ("ada@example.test", 0, 16),
            ("ada@example.test", 17, 33),
            ("grace@example.test", 34, 52),
        ],
        &[",", ";"],
    );
}

#[test]
fn trailing_punctuation_control() {
    assert_emails(
        "Contact: (ada@example.test).",
        &[("ada@example.test", 10, 26)],
        &["Contact: (", ")."],
    );
}

#[test]
fn malformed_and_unsupported_tokens_are_not_salvaged() {
    for value in [
        "missing-at.example",
        "ada@localhost",
        "@example.test",
        "ada@",
        "éada@example.test",
        "\"ada\"@example.test",
        "ada@example.test/path",
        "ada@example.test!",
        "ada@example.test?query",
        "ada@example.test|tail",
    ] {
        assert_emails(value, &[], &[value]);
        let adjacent = format!("Note,{value};end");
        assert_emails(&adjacent, &[], &[&adjacent]);
    }
}

#[test]
fn supported_punctuation_boundaries_keep_email_internal_characters() {
    for delimiter in [',', ';', ':', '(', ')', '[', ']', '<', '>'] {
        let text = format!("Note{delimiter}a.b_c%tag+list-d@example.test{delimiter}end");
        assert_emails(
            &text,
            &[("a.b_c%tag+list-d@example.test", 5, 34)],
            &[&text[..5], &text[34..]],
        );
    }
}

#[test]
fn email_boundaries_do_not_split_decimal_or_phone_tokens() {
    let text = "12.50 +1-202-555-0100 12.50,+1-202-555-0100";
    let parsed = parse_text_with_assignment(text, &[], &[]);
    assert_eq!(parsed.candidates.len(), 2);
    assert_eq!(parsed.candidates[0].candidate_type, CandidateType::Decimal);
    assert_eq!(parsed.candidates[0].raw_value, "12.50");
    assert_eq!(
        parsed.candidates[0].source_span,
        TextSpan {
            byte_start: 0,
            byte_end: 5
        }
    );
    assert_eq!(
        parsed.candidates[1].candidate_type,
        CandidateType::PhoneNumber
    );
    assert_eq!(parsed.candidates[1].raw_value, "+1-202-555-0100");
    assert_eq!(
        parsed.candidates[1].source_span,
        TextSpan {
            byte_start: 6,
            byte_end: 21
        }
    );
    assert_eq!(parsed.assignment.unassigned_candidates, parsed.candidates);
}

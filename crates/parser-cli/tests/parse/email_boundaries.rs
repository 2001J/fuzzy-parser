use super::*;

fn assert_email_output(text: &str, expected: &[(&str, usize, usize)], unused: &[&str]) {
    let output = parse_stdin_content(text);
    assert_eq!(output.stdout, parse_stdin_content(text).stdout);
    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(response["source_type"], "stdin");
    assert_eq!(response["content"]["mode"], "text");
    assert_eq!(response["content"]["records"].as_array().unwrap().len(), 1);
    let document = &response["source_evidence"]["document"];
    assert!(document["source"]["delimiter"].is_null());
    assert_eq!(document["blocks"].as_array().unwrap().len(), 1);
    assert_eq!(document["blocks"][0]["value"]["value"], text);
    assert_eq!(document["blocks"][0]["location"]["byte_start"], 0);
    assert_eq!(document["blocks"][0]["location"]["byte_end"], text.len());
    let parse = &response["content"]["records"][0]["parse"];
    let candidates = parse["candidates"].as_array().unwrap();
    assert_eq!(candidates.len(), expected.len(), "{text:?}");
    for (candidate, &(raw, start, end)) in candidates.iter().zip(expected) {
        let span = serde_json::json!({"byte_start": start, "byte_end": end});
        assert_eq!(candidate["candidate_type"], "email");
        assert_eq!(candidate["raw_value"], raw);
        assert_eq!(candidate["normalized_value"], raw.to_ascii_lowercase());
        assert_eq!(candidate["source_span"], span);
        assert_eq!(
            candidate["source_reference"],
            serde_json::json!({
                "block_index": 0, "coordinate_space": "raw_text_utf8", "span": span
            })
        );
    }
    let assignment = &parse["assignment"];
    let codes: Vec<_> = assignment["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|warning| warning["code"].as_str().unwrap())
        .collect();
    assert_eq!(
        codes,
        match expected.len() {
            0 => vec!["required_field_missing"],
            1 => vec![],
            _ => vec!["multiple_candidates_ambiguous"],
        }
    );
    if candidates.is_empty() {
        assert_eq!(assignment["fields"], serde_json::json!([]));
        assert_eq!(assignment["unassigned_candidates"], serde_json::json!([]));
    } else {
        assert_candidate_sources(&response);
        assert_eq!(
            assignment["fields"][0]["candidates"],
            serde_json::json!([candidates[0]])
        );
        assert_eq!(
            assignment["unassigned_candidates"],
            serde_json::json!(candidates[1..])
        );
    }
    let unused_spans = response["source_evidence"]["blocks"][0]["unused_spans"]
        .as_array()
        .unwrap();
    let unused_text: Vec<_> = unused_spans
        .iter()
        .map(|span| {
            &text[span["byte_start"].as_u64().unwrap() as usize
                ..span["byte_end"].as_u64().unwrap() as usize]
        })
        .collect();
    assert_eq!(unused_text, unused);
    assert_eq!(parse["review"]["status"], "needs_review");
}

#[test]
fn comma_adjacent_email_has_exact_source_and_no_missing_warning() {
    assert_email_output(
        "Ada Lovelace,ada@example.test",
        &[("ada@example.test", 13, 29)],
        &["Ada Lovelace,"],
    );
}

#[test]
fn spaced_email_control() {
    assert_email_output(
        "Ada Lovelace ada@example.test",
        &[("ada@example.test", 13, 29)],
        &["Ada Lovelace "],
    );
}

#[test]
fn unicode_prefix_keeps_byte_coordinates() {
    assert_email_output(
        "Zoë 東京,ADA@example.test",
        &[("ADA@example.test", 12, 28)],
        &["Zoë 東京,"],
    );
}

#[test]
fn multiple_addresses_preserve_unassigned_copy_and_ambiguity() {
    assert_email_output(
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
    assert_email_output(
        "Contact: (ada@example.test).",
        &[("ada@example.test", 10, 26)],
        &["Contact: (", ")."],
    );
}

#[test]
fn malformed_addresses_remain_raw_with_required_warning() {
    for value in [
        "missing-at.example",
        "ada@localhost",
        "@example.test",
        "ada@",
        "éada@example.test",
        "\"ada\"@example.test",
        "ada@example.test/path",
    ] {
        let text = format!("Note,{value};end");
        assert_email_output(&text, &[], &[&text]);
    }
}

#[test]
fn multiline_punctuation_keeps_block_and_input_offsets_distinct() {
    let text = "note\r\n東京,ada@example.test;\n";
    let output = parse_stdin_content(text);
    assert_eq!(output.stdout, parse_stdin_content(text).stdout);
    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_candidate_sources(&response);
    let blocks = response["source_evidence"]["document"]["blocks"]
        .as_array()
        .unwrap();
    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[1]["value"]["value"], "東京,ada@example.test;");
    assert_eq!(blocks[1]["location"]["byte_start"], 6);
    let parse = &response["content"]["records"][1]["parse"];
    let candidate = &parse["candidates"][0];
    let span = serde_json::json!({"byte_start": 7, "byte_end": 23});
    assert_eq!(candidate["raw_value"], "ada@example.test");
    assert_eq!(candidate["source_span"], span);
    assert_eq!(
        candidate["source_reference"],
        serde_json::json!({
            "block_index": 1, "coordinate_space": "raw_text_utf8", "span": span
        })
    );
    assert_eq!(&text.as_bytes()[13..29], b"ada@example.test");
    assert_eq!(
        parse["assignment"]["fields"][0]["candidates"][0],
        *candidate
    );
    assert_eq!(parse["assignment"]["warnings"], serde_json::json!([]));
    assert_eq!(
        response["content"]["records"][0]["parse"]["assignment"]["warnings"][0]["code"],
        "required_field_missing"
    );
}

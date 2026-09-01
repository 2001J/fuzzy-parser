use super::support;

fn schema_bytes(size: usize) -> Vec<u8> {
    let mut input = br#"{"schema_version":"0.1","record_name":null,"fields":[],"options":{"allow_unknown_fields":true}}"#.to_vec();
    assert!(input.len() <= size);
    input.resize(size, b' ');
    input
}

fn assert_schema_byte_limit(output: &std::process::Output, limit: usize) {
    let report = support::error(output);
    assert_eq!(
        report,
        serde_json::json!({
            "error": {
                "error_contract_version": "0.1",
                "code": "resource_limit",
                "resource": "schema_bytes",
                "limit": limit,
                "actual": limit + 1
            },
            "message": format!(
                "resource limit schema_bytes exceeded: limit {limit}, actual {}",
                limit + 1
            )
        })
    );
}

#[test]
fn schema_file_and_stdin_reads_accept_exact_default_and_stop_at_one_over() {
    let limit = parser_schema::SchemaLimits::default().max_bytes;
    let exact = schema_bytes(limit);
    let one_over = schema_bytes(limit + 1);
    let directory = support::TestDirectory::new();
    let exact_path = directory.file("exact.json", &exact);
    let over_path = directory.file("over.json", &one_over);

    let file_exact = support::run(&["schema", "validate", exact_path.to_str().unwrap()], None);
    assert_eq!(file_exact.status.code(), Some(0), "{file_exact:?}");
    assert!(file_exact.stderr.is_empty());
    assert_schema_byte_limit(
        &support::run(&["schema", "validate", over_path.to_str().unwrap()], None),
        limit,
    );

    let stdin_exact = support::run(&["schema", "validate", "--stdin"], Some(&exact));
    assert_eq!(stdin_exact.status.code(), Some(0), "{stdin_exact:?}");
    assert!(stdin_exact.stderr.is_empty());
    assert_schema_byte_limit(
        &support::run(&["schema", "validate", "--stdin"], Some(&one_over)),
        limit,
    );
}

#[test]
fn parse_schema_file_is_bounded_before_input_extraction() {
    let limit = parser_schema::SchemaLimits::default().max_bytes;
    let directory = support::TestDirectory::new();
    let input = directory.file("input.txt", b"synthetic");
    let exact = directory.file("exact.json", &schema_bytes(limit));
    let over = directory.file("over.json", &schema_bytes(limit + 1));

    let accepted = support::run(
        &[
            "parse",
            input.to_str().unwrap(),
            "--schema",
            exact.to_str().unwrap(),
        ],
        None,
    );
    assert_eq!(accepted.status.code(), Some(0), "{accepted:?}");
    assert!(accepted.stderr.is_empty());

    let missing_input = directory.0.join("missing.txt");
    let rejected = support::run(
        &[
            "parse",
            missing_input.to_str().unwrap(),
            "--schema",
            over.to_str().unwrap(),
        ],
        None,
    );
    assert_schema_byte_limit(&rejected, limit);
}

#[test]
fn schema_validate_reports_default_nesting_limit_before_structural_decode() {
    let limit = parser_schema::SchemaLimits::default().max_nesting;
    let mut schema = br#"{"schema_version":"0.1","record_name":null,"fields":[],"options":{"allow_unknown_fields":true},"extra":"#.to_vec();
    schema.extend(std::iter::repeat_n(b'[', limit));
    schema.extend_from_slice(b"null");
    schema.extend(std::iter::repeat_n(b']', limit));
    schema.push(b'}');

    let output = support::run(&["schema", "validate", "--stdin"], Some(&schema));
    let report = support::error(&output);
    assert_eq!(
        report,
        serde_json::json!({
            "error": {
                "error_contract_version": "0.1",
                "code": "resource_limit",
                "resource": "schema_nesting",
                "limit": limit,
                "actual": limit + 1
            },
            "message": format!(
                "resource limit schema_nesting exceeded: limit {limit}, actual {}",
                limit + 1
            )
        })
    );
}

#[test]
fn inspect_response_limit_is_a_processing_failure_without_partial_stdout() {
    let directory = support::TestDirectory::new();
    let row = format!("{},{}\n", "x".repeat(120), "y".repeat(120));
    let csv = row.repeat(50_000);
    assert!(csv.len() < parser_formats::CsvLimits::default().max_bytes as usize);
    let path = directory.file("large.csv", csv.as_bytes());

    let output = support::run(&["inspect", path.to_str().unwrap()], None);
    let report = support::error(&output);
    let limit = parser_core::ParseLimits::default().max_response_bytes;
    assert_eq!(
        report,
        serde_json::json!({
            "error": {
                "error_contract_version": "0.1",
                "code": "resource_limit",
                "resource": "response_bytes",
                "limit": limit,
                "actual": limit + 1
            },
            "message": format!(
                "resource limit response_bytes exceeded: limit {limit}, actual {}",
                limit + 1
            )
        })
    );
}

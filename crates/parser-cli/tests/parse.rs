use serde_json::Value;
mod support;
use std::{
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

#[path = "parse/email_boundaries.rs"]
mod email_boundaries;

#[path = "parse/conformance.rs"]
mod conformance;

#[path = "parse/schema_compilation.rs"]
mod schema_compilation;

#[path = "parse/resource_limits.rs"]
mod resource_limits;

#[path = "parse/table_selection.rs"]
mod table_selection;

#[path = "parse/text_names.rs"]
mod text_names;

#[path = "parse/text_pipeline.rs"]
mod text_pipeline;

fn csv_fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/csv/comma.csv")
}

fn text_fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/text/simple.txt")
}

fn schema_fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/schema/contact.json")
}

#[test]
fn parse_unsupported_types_expose_only_known_type_literals_by_default() {
    let directory = support::TestDirectory::new();
    let input = directory.file("private.txt", b"private input --diagnostics");
    for field_type in ["text", "person_name", "datetime"] {
        let schema = serde_json::json!({
            "schema_version": "0.1", "record_name": "private record", "options": {"allow_unknown_fields":true},
            "fields": [{"name":"private field 東京\n\u{1b}", "field_type": field_type, "required":false, "multiple":false, "aliases":[], "constraints":[]}]
        });
        let path = directory.file(
            "private-schema.json",
            serde_json::to_string(&schema).unwrap().as_bytes(),
        );
        let args = [
            "parse",
            input.to_str().unwrap(),
            "--schema",
            path.to_str().unwrap(),
        ];
        let safe = support::run(&args, None);
        // #13 makes text/name executable; their historical error payloads remain
        // covered by the core serialization tests. Datetime is still rejected.
        if field_type != "datetime" {
            assert_eq!(safe.status.code(), Some(0));
            assert!(safe.stderr.is_empty());
            let response: Value = serde_json::from_slice(&safe.stdout).unwrap();
            let parsed = &response["content"]["records"][0]["parse"];
            assert!(
                parsed["assignment"]["fields"]
                    .as_array()
                    .unwrap()
                    .is_empty()
            );
            assert!(
                !parsed["assignment"]["unassigned_candidates"]
                    .as_array()
                    .unwrap()
                    .is_empty()
            );
            assert_eq!(parsed["review"]["status"], "needs_review");
            assert_candidate_sources(&response);
            let mut detailed = args.to_vec();
            detailed.insert(0, "--diagnostics");
            let repeated = support::run(&detailed, None);
            assert_eq!(repeated.status.code(), Some(0));
            assert!(repeated.stderr.is_empty());
            assert_eq!(repeated.stdout, safe.stdout);
            continue;
        }
        let mut expected = serde_json::json!({"error_contract_version":"0.1", "code":"schema_field_type_unsupported", "field_type":field_type});
        assert_eq!(
            support::error(&safe),
            serde_json::json!({"error":expected, "message":format!("field type \"{field_type}\" is not supported by the parser yet")})
        );
        assert!(!String::from_utf8_lossy(&safe.stderr).contains("private"));
        let mut trailing = args.to_vec();
        trailing.push("--diagnostics");
        // #6 rejects the formerly ignored tail without enabling diagnostics.
        let usage = support::run(&trailing, None);
        assert_eq!(usage.status.code(), Some(2));
        assert!(usage.stdout.is_empty());
        assert_eq!(usage.stderr, b"usage: parser-cli --help\n");
        expected["diagnostics"] = serde_json::json!({"field":"private field 東京\n\u{1b}"});
        let mut detailed_args = args.to_vec();
        detailed_args.insert(0, "--diagnostics");
        let detailed = support::run(&detailed_args, None);
        assert_eq!(support::error(&detailed)["error"], expected);
        assert_eq!(detailed.stderr, support::run(&detailed_args, None).stderr);
        assert!(!String::from_utf8_lossy(&detailed.stderr).contains("private input"));
        assert!(!String::from_utf8_lossy(&detailed.stderr).contains('\u{1b}'));
    }
}

#[test]
fn parse_file_errors_share_the_inspect_and_schema_error_boundary() {
    let directory = support::TestDirectory::new();
    let schema = schema_fixture_path();
    for (name, bytes, code) in [
        ("private.txt", b"private\xff".as_slice(), "invalid_utf8"),
        (
            "private.csv",
            b"name,note\nprivate,\"unclosed".as_slice(),
            "invalid_csv",
        ),
        (
            "private.xlsx",
            b"private workbook data".as_slice(),
            "invalid_xlsx",
        ),
    ] {
        let path = directory.file(name, bytes);
        for prefix in [vec![], vec!["--diagnostics"]] {
            let mut parse = prefix.clone();
            parse.extend([
                "parse",
                path.to_str().unwrap(),
                "--schema",
                schema.to_str().unwrap(),
            ]);
            let output = support::run(&parse, None);
            assert_eq!(support::error(&output)["error"]["code"], code);
            let mut inspect = prefix;
            inspect.extend(["inspect", path.to_str().unwrap()]);
            assert_eq!(output.stderr, support::run(&inspect, None).stderr);
        }
    }
    for bytes in [b"{private schema".as_slice(), b"private\xff".as_slice()] {
        let schema = directory.file("private.json", bytes);
        for prefix in [vec![], vec!["--diagnostics"]] {
            let mut parse = prefix.clone();
            parse.extend([
                "parse",
                "missing-input.txt",
                "--schema",
                schema.to_str().unwrap(),
            ]);
            let output = support::run(&parse, None);
            support::error(&output);
            let mut validate = prefix;
            validate.extend(["schema", "validate", schema.to_str().unwrap()]);
            assert_eq!(output.stderr, support::run(&validate, None).stderr);
        }
    }
}

#[test]
fn leading_diagnostics_does_not_change_stdin_parse_source_or_review_output() {
    let input = "--diagnostics 東京\nada@example.test\n\n";
    let schema = schema_fixture_path();
    let args = ["parse", "--stdin", "--schema", schema.to_str().unwrap()];
    let safe = support::run(&args, Some(input.as_bytes()));
    let detailed = support::run(
        &[
            "--diagnostics",
            "parse",
            "--stdin",
            "--schema",
            schema.to_str().unwrap(),
        ],
        Some(input.as_bytes()),
    );
    assert_eq!(safe.status.code(), Some(0));
    assert_eq!(detailed.status.code(), Some(0));
    assert!(safe.stderr.is_empty());
    assert!(detailed.stderr.is_empty());
    assert_eq!(safe.stdout, detailed.stdout);
    let response: Value = serde_json::from_slice(&safe.stdout).unwrap();
    assert_candidate_sources(&response);
    assert_eq!(
        response["source_evidence"]["document"]["blocks"][0]["value"]["value"],
        "--diagnostics 東京"
    );
}

fn text_schema_fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/schema/contact_with_text.json")
}

fn run_parse(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_parser-cli"))
        .arg("parse")
        .args(args)
        .output()
        .expect("CLI should run")
}

fn parse_stdin_content(content: &str) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_parser-cli"))
        .args([
            "parse",
            "--stdin",
            "--schema",
            schema_fixture_path().to_str().unwrap(),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(content.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(0), "{:?}", output.stderr);
    assert!(output.stderr.is_empty());
    output
}

fn assert_candidate_sources(response: &Value) {
    let typed: parser_core::ParseResponse = serde_json::from_value(response.clone()).unwrap();
    let document = &typed.source_evidence.as_ref().unwrap().document;
    fn visit(value: &Value, document: &parser_core::RawDocument) -> usize {
        match value {
            Value::Object(object) => {
                if object.contains_key("candidate_type") {
                    let reference: parser_core::SourceReference =
                        serde_json::from_value(object["source_reference"].clone()).unwrap();
                    assert_eq!(
                        reference.resolve(document).as_deref(),
                        object["raw_value"].as_str()
                    );
                    return 1;
                }
                object.values().map(|value| visit(value, document)).sum()
            }
            Value::Array(values) => values.iter().map(|value| visit(value, document)).sum(),
            _ => 0,
        }
    }
    assert!(
        visit(response, document) > 0,
        "exercise candidate copies in serialized output"
    );
}

#[test]
fn parse_unicode_without_candidates_matches_source_review_golden() {
    let content = "  Zoë — 東京  ";
    let first = parse_stdin_content(content);
    let second = parse_stdin_content(content);
    assert_eq!(first.stdout, second.stdout);
    let response: Value = serde_json::from_slice(&first.stdout).unwrap();
    let golden: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/contracts/parse-source-review.json"
    )))
    .unwrap();
    assert_eq!(response, golden);
    assert_eq!(
        response["source_evidence"]["document"]["blocks"][0]["value"]["value"],
        content
    );
    assert_eq!(
        response["source_evidence"]["blocks"][0]["unused_spans"][0]["byte_end"],
        content.len()
    );
}

fn assert_context_stdin_output(prefix: &str, multiple_candidates: bool) {
    let suffix = if multiple_candidates {
        " ada@example.test grace@example.test"
    } else {
        " ada@example.test"
    };
    let content = format!("{prefix}{suffix}");
    let output = parse_stdin_content(&content);
    assert_eq!(output.stdout, parse_stdin_content(&content).stdout);
    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_candidate_sources(&response);
    assert_eq!(
        response["source_evidence"]["document"]["blocks"][0]["value"]["value"],
        content
    );
    let parse = &response["content"]["records"][0]["parse"];
    let candidates = parse["candidates"].as_array().unwrap();
    assert_eq!(candidates.len(), if multiple_candidates { 2 } else { 1 });
    assert_eq!(
        parse["assignment"]["fields"][0]["candidates"],
        serde_json::json!([candidates[0]])
    );
    assert_eq!(candidates[0]["raw_value"], "ada@example.test");
    assert_eq!(
        parse["assignment"]["unassigned_candidates"],
        serde_json::json!(candidates[1..])
    );
    for candidate in candidates {
        let raw = candidate["raw_value"].as_str().unwrap();
        let start = content.find(raw).unwrap();
        let span = serde_json::json!({"byte_start": start, "byte_end": start + raw.len()});
        assert_eq!(candidate["source_span"], span);
        assert_eq!(
            candidate["source_reference"],
            serde_json::json!({"block_index": 0, "coordinate_space": "raw_text_utf8", "span": span})
        );
    }
    let codes: Vec<_> = parse["assignment"]["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|warning| warning["code"].as_str().unwrap())
        .collect();
    assert_eq!(
        codes,
        if multiple_candidates {
            vec!["multiple_candidates_ambiguous"]
        } else {
            vec![]
        }
    );
    assert_eq!(parse["review"]["status"], "needs_review");
}

#[test]
fn parse_assignment_context_two_byte_prefix() {
    assert_context_stdin_output(&"é".repeat(21), true);
}

#[test]
fn parse_assignment_context_three_byte_prefix() {
    assert_context_stdin_output(&"東京".repeat(15), true);
}

#[test]
fn parse_assignment_context_four_byte_prefix() {
    assert_context_stdin_output(&"😀".repeat(11), true);
}

#[test]
fn parse_assignment_context_ascii_and_single_candidate_controls() {
    assert_context_stdin_output(&"x".repeat(42), true);
    for prefix in ["é".repeat(21), "東京".repeat(15), "😀".repeat(11)] {
        assert_context_stdin_output(&prefix, false);
    }
}

#[test]
fn parse_assignment_context_csv_preserves_unicode_cells() {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/csv/unicode-assignment.csv");
    let schema_path = schema_fixture_path();
    let args = [
        path.to_str().unwrap(),
        "--schema",
        schema_path.to_str().unwrap(),
    ];
    let output = run_parse(&args);
    assert_eq!(output.status.code(), Some(0), "{:?}", output.stderr);
    assert!(output.stderr.is_empty());
    let repeated = run_parse(&args);
    assert_eq!(repeated.status.code(), Some(0));
    assert!(repeated.stderr.is_empty());
    assert_eq!(output.stdout, repeated.stdout);
    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_candidate_sources(&response);
    let sheet = &response["content"]["sheets"][0];
    assert_eq!(sheet["header"]["status"], "detected");
    assert_eq!(sheet["records"].as_array().unwrap().len(), 3);
    for (index, prefix) in ["é".repeat(21), "東京".repeat(15), "😀".repeat(11)]
        .iter()
        .enumerate()
    {
        let block_index = 3 * (index + 1);
        let blocks = &response["source_evidence"]["document"]["blocks"];
        assert_eq!(
            blocks[block_index]["value"]["value"],
            format!("  {prefix}  ")
        );
        let parse = &sheet["records"][index]["parse"];
        assert_eq!(parse["candidates"].as_array().unwrap().len(), 2);
        let assignment = &parse["assignment"];
        assert_eq!(
            assignment["fields"][0]["candidates"][0],
            parse["candidates"][0]
        );
        assert_eq!(
            assignment["unassigned_candidates"][0],
            parse["candidates"][1]
        );
        assert_eq!(
            assignment["warnings"][0]["code"],
            "multiple_candidates_ambiguous"
        );
        assert_eq!(parse["review"]["status"], "needs_review");
        for (column, raw) in ["ada@example.test", "grace@example.test"]
            .iter()
            .enumerate()
        {
            let candidate = &parse["candidates"][column];
            assert_eq!(candidate["raw_value"], *raw);
            assert_eq!(
                blocks[block_index + column + 1]["value"]["value"],
                format!(" {raw} ")
            );
            assert_eq!(
                candidate["source_reference"],
                serde_json::json!({"block_index": block_index + column + 1, "coordinate_space": "raw_text_utf8", "span": {"byte_start": 1, "byte_end": 1 + raw.len()}})
            );
        }
    }
}

#[test]
fn additive_source_extension_preserves_every_legacy_golden_field() {
    let output = parse_stdin_content("ada@example.test 42");
    let mut response: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_candidate_sources(&response);
    fn remove_additions(value: &mut Value) {
        match value {
            Value::Object(object) => {
                for key in ["source_evidence", "source_reference", "review"] {
                    object.remove(key);
                }
                for value in object.values_mut() {
                    remove_additions(value);
                }
            }
            Value::Array(values) => {
                for value in values {
                    remove_additions(value);
                }
            }
            _ => {}
        }
    }
    remove_additions(&mut response);
    let old: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/contracts/parse-0.1.json"
    )))
    .unwrap();
    assert_eq!(response, old);
}

#[test]
fn parse_embeds_exact_inspect_values_for_text_csv_and_typed_xlsx() {
    for relative in [
        "text/simple.txt",
        "csv/source-review.csv",
        "xlsx/sample.xlsx",
    ] {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures")
            .join(relative);
        let inspect = Command::new(env!("CARGO_BIN_EXE_parser-cli"))
            .args(["inspect", path.to_str().unwrap()])
            .output()
            .unwrap();
        assert!(inspect.status.success());
        assert!(inspect.stderr.is_empty());
        let original: Value = serde_json::from_slice(&inspect.stdout).unwrap();
        let output = run_parse(&[
            path.to_str().unwrap(),
            "--schema",
            schema_fixture_path().to_str().unwrap(),
        ]);
        assert_eq!(output.status.code(), Some(0));
        assert!(output.stderr.is_empty());
        let response: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(response["source_evidence"]["document"], original);
        assert_eq!(
            response["source_evidence"]["blocks"]
                .as_array()
                .unwrap()
                .len(),
            original["blocks"].as_array().unwrap().len()
        );
        if relative.ends_with(".csv") {
            assert_candidate_sources(&response);
            let records = &response["content"]["sheets"][0]["records"];
            assert_eq!(records.as_array().unwrap().len(), 2);
            let email = &records[0]["parse"]["assignment"]["fields"][0]["candidates"][0];
            assert_eq!(
                email["source_reference"],
                serde_json::json!({"block_index": 5, "coordinate_space": "raw_text_utf8", "span": {"byte_start": 2, "byte_end": 18}})
            );
            assert_eq!(
                email["source_span"],
                serde_json::json!({"byte_start": 5, "byte_end": 21})
            );
            assert_eq!(response["source_evidence"]["blocks"][0]["role"], "header");
            assert_eq!(original["blocks"][4]["value"]["value"], "  Zoë  ");
            assert_eq!(original["blocks"][6]["value"]["value"], "");
            assert_eq!(original["blocks"][10]["value"]["value"], "  ");
            assert_eq!(
                records[1]["parse"]["assignment"]["warnings"][0]["code"],
                "required_field_missing"
            );
            assert_eq!(records[1]["parse"]["review"]["status"], "needs_review");
        } else if relative.ends_with(".xlsx") {
            assert_candidate_sources(&response);
            assert_eq!(
                original["blocks"][5]["value"],
                serde_json::json!({"kind": "Decimal", "value": 42.0})
            );
            assert_eq!(
                original["blocks"][6]["value"],
                serde_json::json!({"kind": "Boolean", "value": true})
            );
            assert_eq!(
                original["blocks"][7]["value"],
                serde_json::json!({"kind": "DateTime", "value": 45943.5})
            );
            assert_eq!(
                original["blocks"][10]["value"],
                serde_json::json!({"kind": "Null"})
            );
            assert_eq!(original["blocks"][5]["location"]["byte_start"], Value::Null);
            assert_eq!(
                response["source_evidence"]["blocks"][5]["coordinate_space"],
                "rendered_value_utf8"
            );
            assert_eq!(
                response["source_evidence"]["blocks"][10]["unused_spans"],
                serde_json::json!([{"byte_start": 0, "byte_end": 0}])
            );
        }
    }
}

#[test]
fn parse_csv_with_schema_assigns_by_header() {
    let output = run_parse(&[
        csv_fixture_path().to_str().expect("CSV path is UTF-8"),
        "--schema",
        schema_fixture_path()
            .to_str()
            .expect("schema path is UTF-8"),
    ]);

    assert!(output.status.success(), "{:?}", output.stderr);
    assert!(output.stderr.is_empty());

    let response: Value = serde_json::from_slice(&output.stdout).expect("stdout should be JSON");
    assert_eq!(response["contract_version"], "0.1");
    assert_eq!(response["parser_version"], "0.1.0");
    assert_eq!(response["record_name"], "contact");
    assert_eq!(response["source_type"], "csv");
    assert_eq!(response["content"]["mode"], "table");
    assert_eq!(
        response["content"]["sheets"].as_array().map(Vec::len),
        Some(1)
    );

    let sheet = &response["content"]["sheets"][0];
    assert_eq!(sheet["header"]["status"], "detected");
    assert_eq!(sheet["records"].as_array().map(Vec::len), Some(2));

    let first_record = &sheet["records"][0]["parse"]["assignment"];
    assert_eq!(first_record["fields"][0]["name"], "email");
    assert_eq!(
        first_record["fields"][0]["candidates"][0]["raw_value"],
        "ada@example.test"
    );
    assert_eq!(
        first_record["fields"][0]["candidates"][0]["source_column"],
        2
    );
}

#[test]
fn parse_text_with_schema_reports_required_missing() {
    let output = run_parse(&[
        text_fixture_path().to_str().expect("text path is UTF-8"),
        "--schema",
        schema_fixture_path()
            .to_str()
            .expect("schema path is UTF-8"),
    ]);

    assert!(output.status.success(), "{:?}", output.stderr);

    let response: Value = serde_json::from_slice(&output.stdout).expect("stdout should be JSON");
    assert_eq!(response["source_type"], "txt");
    assert_eq!(response["content"]["mode"], "text");
    assert_eq!(
        response["content"]["records"].as_array().map(Vec::len),
        Some(2)
    );
    let first = &response["content"]["records"][0]["parse"]["assignment"];
    assert!(first["warnings"].as_array().is_some_and(|warnings| {
        warnings
            .iter()
            .any(|warning| warning["code"] == "required_field_missing")
    }));
}

#[test]
fn parse_rejects_unsupported_field_type_as_structured_error() {
    let directory = support::TestDirectory::new();
    let mut schema: Value =
        serde_json::from_slice(&std::fs::read(text_schema_fixture_path()).unwrap()).unwrap();
    schema["fields"][0]["field_type"] = "datetime".into();
    let schema_path = directory.file("datetime.json", schema.to_string().as_bytes());
    let output = run_parse(&[
        csv_fixture_path().to_str().expect("CSV path is UTF-8"),
        "--schema",
        schema_path.to_str().expect("schema path is UTF-8"),
    ]);

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let error: Value = serde_json::from_slice(&output.stderr).expect("stderr should be JSON");
    assert_eq!(error["error"]["code"], "schema_field_type_unsupported");
    assert!(
        error["message"]
            .as_str()
            .is_some_and(|message| message.contains("datetime"))
    );
}

#[test]
fn parse_reports_missing_input_as_structured_error() {
    let output = run_parse(&[
        "fixtures/text/does-not-exist.txt",
        "--schema",
        schema_fixture_path()
            .to_str()
            .expect("schema path is UTF-8"),
    ]);

    assert_eq!(output.status.code(), Some(1));
    let error: Value = serde_json::from_slice(&output.stderr).expect("stderr should be JSON");
    assert_eq!(error["error"]["code"], "io_error");
}

#[test]
fn parse_reports_missing_schema_as_structured_error() {
    let output = run_parse(&[
        csv_fixture_path().to_str().expect("CSV path is UTF-8"),
        "--schema",
        "fixtures/schema/does-not-exist.json",
    ]);

    assert_eq!(output.status.code(), Some(1));
    let error: Value = serde_json::from_slice(&output.stderr).expect("stderr should be JSON");
    assert_eq!(error["error"]["code"], "schema_io_error");
}

#[test]
fn parse_accepts_stdin_with_schema() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_parser-cli"))
        .args([
            "parse",
            "--stdin",
            "--schema",
            schema_fixture_path()
                .to_str()
                .expect("schema path is UTF-8"),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("CLI should run");
    child
        .stdin
        .take()
        .expect("stdin should be available")
        .write_all(b"hello ada@example.test")
        .expect("input should be written");

    let output = child.wait_with_output().expect("CLI should finish");

    assert!(output.status.success(), "{:?}", output.stderr);
    let response: Value = serde_json::from_slice(&output.stdout).expect("stdout should be JSON");
    assert_eq!(response["source_type"], "stdin");
    assert_eq!(response["content"]["mode"], "text");
    assert_eq!(
        response["content"]["records"][0]["parse"]["assignment"]["fields"][0]["candidates"][0]["raw_value"],
        "ada@example.test"
    );
}

#[test]
fn parse_requires_a_schema_flag() {
    let output = run_parse(&["somefile.csv"]);

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(output.stderr, b"usage: parser-cli --help\n");
}

#[test]
fn parse_help_lists_parse_modes() {
    let output = Command::new(env!("CARGO_BIN_EXE_parser-cli"))
        .args(["parse", "--help"])
        .output()
        .expect("CLI should run");

    assert!(output.status.success());
    let help = String::from_utf8_lossy(&output.stdout);
    assert!(help.contains("parse <path> --schema"));
    assert!(help.contains("parse --stdin --schema"));
}

use super::support;
use serde_json::{Value, json};
use std::{fs, path::PathBuf};

fn fixture(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(path)
}

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

fn schema(text_pipeline: Value) -> Value {
    json!({
        "schema_version": "0.1",
        "record_name": "synthetic",
        "fields": [{
            "name": "email",
            "field_type": "email",
            "required": true,
            "multiple": false,
            "aliases": [],
            "constraints": []
        }],
        "options": {
            "allow_unknown_fields": true,
            "text_pipeline": text_pipeline
        }
    })
}

fn run_stdin(profile: &Value, input: &[u8]) -> (Value, Vec<u8>) {
    let directory = support::TestDirectory::new();
    let path = directory.file("schema.json", profile.to_string().as_bytes());
    let output = support::run(
        &["parse", "--stdin", "--schema", path.to_str().unwrap()],
        Some(input),
    );
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stderr.is_empty());
    let repeated = support::run(
        &["parse", "--stdin", "--schema", path.to_str().unwrap()],
        Some(input),
    );
    assert_eq!(output.stdout, repeated.stdout);
    (
        serde_json::from_slice(&output.stdout).unwrap(),
        output.stdout,
    )
}

#[test]
fn opt_in_one_block_exposes_reversible_composition_without_changing_raw_source() {
    let directory = support::TestDirectory::new();
    let schema = directory.file(
        "schema.json",
        schema(pipeline("one_block_per_record", &[]))
            .to_string()
            .as_bytes(),
    );
    let input = "  \u{2014} email: ADA@example.test  ";
    let output = support::run(
        &["parse", "--stdin", "--schema", schema.to_str().unwrap()],
        Some(input.as_bytes()),
    );
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stderr.is_empty());
    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        response["source_evidence"]["document"]["blocks"][0]["value"]["value"],
        input
    );
    let record = &response["content"]["records"][0];
    assert_eq!(record["composition"]["record_id"], "record-1");
    assert_eq!(
        record["composition"]["composed_text"],
        "- email: ADA@example.test"
    );
    assert_eq!(
        record["parse"]["assignment"]["fields"][0]["candidates"][0]["raw_value"],
        "ADA@example.test"
    );
    assert_eq!(
        record["parse"]["assignment"]["fields"][0]["candidates"][0]["source_reference"]["span"],
        json!({"byte_start": 13, "byte_end": 29})
    );
    assert_eq!(
        record["composition"]["segments"].as_array().unwrap().len(),
        1
    );
}

#[test]
fn joined_unicode_text_and_scalar_values_match_the_shared_library_plan() {
    let mut profile = schema(pipeline("join_indented_continuations", &[]));
    profile["fields"] = json!([
        {
            "name": "name", "field_type": "person_name", "required": true,
            "multiple": false, "aliases": [], "constraints": []
        },
        {
            "name": "email", "field_type": "email", "required": true,
            "multiple": false, "aliases": [], "constraints": []
        }
    ]);
    let input = "name: Zoe\u{301}  東京\n  email: ADA@example.test".as_bytes();
    let (response, stdout) = run_stdin(&profile, input);
    let record = &response["content"]["records"][0];
    assert_eq!(
        record["composition"]["composed_text"],
        "name: Zoe\u{301} 東京\nemail: ADA@example.test"
    );
    let fields = record["parse"]["assignment"]["fields"].as_array().unwrap();
    assert_eq!(fields[0]["candidates"][0]["raw_value"], "Zoe\u{301}  東京");
    assert_eq!(
        fields[0]["candidates"][0]["normalized_value"],
        "Zoe\u{301}  東京"
    );
    assert_eq!(fields[1]["candidates"][0]["raw_value"], "ADA@example.test");

    let mut reader = input;
    let document = parser_formats::read_input(
        parser_formats::InputSource::Stdin(&mut reader),
        parser_formats::TextLimits::default(),
    )
    .unwrap();
    let plan = parser_schema::compile_schema_json(&profile.to_string()).unwrap();
    let library = parser_core::parse_document_with_plan(&document, &plan);
    assert_eq!(
        format!("{}\n", serde_json::to_string_pretty(&library).unwrap()).as_bytes(),
        stdout
    );
}

#[test]
fn repeated_identifier_txt_and_stdin_share_composed_content_and_library_execution() {
    let profile = schema(pipeline("split_repeated_identifiers", &["entry:"]));
    let input = b"entry: ada@example.test entry: grace@example.test";
    let (stdin, _) = run_stdin(&profile, input);

    let directory = support::TestDirectory::new();
    let input_path = directory.file("records.txt", input);
    let schema_path = directory.file("schema.json", profile.to_string().as_bytes());
    let output = support::run(
        &[
            "parse",
            input_path.to_str().unwrap(),
            "--schema",
            schema_path.to_str().unwrap(),
        ],
        None,
    );
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stderr.is_empty());
    let txt: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(txt["content"], stdin["content"]);
    assert_eq!(
        txt["source_evidence"]["blocks"],
        stdin["source_evidence"]["blocks"]
    );
    assert_eq!(txt["content"]["records"].as_array().unwrap().len(), 2);

    let document = parser_formats::read_txt(&input_path).unwrap();
    let plan = parser_schema::compile_schema_json(&profile.to_string()).unwrap();
    assert_eq!(
        serde_json::to_value(parser_core::parse_document_with_plan(&document, &plan)).unwrap(),
        txt
    );
}

#[test]
fn table_inputs_keep_content_and_source_and_append_one_not_applied_warning() {
    for input in [fixture("csv/comma.csv"), fixture("xlsx/sample.xlsx")] {
        let mut base: Value =
            serde_json::from_str(&fs::read_to_string(fixture("schema/contact.json")).unwrap())
                .unwrap();
        let directory = support::TestDirectory::new();
        let base_path = directory.file("base.json", base.to_string().as_bytes());
        let base_output = support::run(
            &[
                "parse",
                input.to_str().unwrap(),
                "--schema",
                base_path.to_str().unwrap(),
            ],
            None,
        );
        assert_eq!(base_output.status.code(), Some(0), "{base_output:?}");
        let base_response: Value = serde_json::from_slice(&base_output.stdout).unwrap();

        base["options"]["text_pipeline"] = pipeline("one_block_per_record", &[]);
        let pipeline_path = directory.file("pipeline.json", base.to_string().as_bytes());
        let output = support::run(
            &[
                "parse",
                input.to_str().unwrap(),
                "--schema",
                pipeline_path.to_str().unwrap(),
            ],
            None,
        );
        assert_eq!(output.status.code(), Some(0), "{output:?}");
        assert!(output.stderr.is_empty());
        let response: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(response["content"], base_response["content"]);
        assert_eq!(
            response["source_evidence"],
            base_response["source_evidence"]
        );
        let mut expected_warnings = base_response["warnings"].as_array().unwrap().clone();
        expected_warnings.push(json!({
            "code": "text_pipeline_not_applied",
            "message": "text pipeline options were not applied because the document used the existing table parse path",
            "location": null
        }));
        assert_eq!(response["warnings"], Value::Array(expected_warnings));
        assert!(
            response["content"]["sheets"]
                .as_array()
                .unwrap()
                .iter()
                .flat_map(|sheet| sheet["records"].as_array().unwrap())
                .all(|record| record.get("composition").is_none())
        );
    }
}

#[test]
fn text_pipeline_keeps_decode_extraction_compile_failure_precedence() {
    let directory = support::TestDirectory::new();
    let mut invalid = schema(pipeline("one_block_per_record", &["entry:"]));
    let compile = directory.file("compile.json", invalid.to_string().as_bytes());
    invalid["options"]["text_pipeline"]["normalization"]["private"] = json!(true);
    let decode = directory.file("decode.json", invalid.to_string().as_bytes());
    let missing = directory.0.join("missing.pdf");
    for (schema, code) in [
        (&decode, "schema_property_unsupported"),
        (&compile, "unsupported_input"),
    ] {
        let output = support::run(
            &[
                "parse",
                missing.to_str().unwrap(),
                "--schema",
                schema.to_str().unwrap(),
            ],
            None,
        );
        assert_eq!(support::error(&output)["error"]["code"], code);
    }
    let input = directory.file("input.txt", b"email: ada@example.test");
    let output = support::run(
        &[
            "parse",
            input.to_str().unwrap(),
            "--schema",
            compile.to_str().unwrap(),
        ],
        None,
    );
    assert_eq!(
        support::error(&output)["error"]["code"],
        "schema_option_unsupported"
    );
}

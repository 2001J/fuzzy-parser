use serde_json::Value;
use std::{
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

fn csv_fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/csv/comma.csv")
}

fn text_fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/text/simple.txt")
}

fn schema_fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/schema/contact.json")
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
    let output = run_parse(&[
        csv_fixture_path().to_str().expect("CSV path is UTF-8"),
        "--schema",
        text_schema_fixture_path()
            .to_str()
            .expect("schema path is UTF-8"),
    ]);

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let error: Value = serde_json::from_slice(&output.stderr).expect("stderr should be JSON");
    assert_eq!(error["error"]["code"], "schema_field_type_unsupported");
    assert!(
        error["message"]
            .as_str()
            .is_some_and(|message| message.contains("text"))
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
    assert!(String::from_utf8_lossy(&output.stderr).contains("parse <path> --schema"));
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

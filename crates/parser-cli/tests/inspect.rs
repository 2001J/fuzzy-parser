use serde_json::Value;
use std::{
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/text/simple.txt")
}

fn csv_fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/csv/comma.csv")
}

fn xlsx_fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/xlsx/sample.xlsx")
}

fn schema_fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/schema/contact.json")
}

#[test]
fn inspect_outputs_canonical_document_json() {
    let output = Command::new(env!("CARGO_BIN_EXE_parser-cli"))
        .args([
            "inspect",
            fixture_path().to_str().expect("fixture path is UTF-8"),
        ])
        .output()
        .expect("CLI should run");

    assert!(output.status.success());
    assert!(output.stderr.is_empty());

    let document: Value = serde_json::from_slice(&output.stdout).expect("stdout should be JSON");
    assert_eq!(document["source"]["source_type"], "txt");
    assert_eq!(document["blocks"][0]["value"]["value"], "Ada Lovelace");
    assert_eq!(document["blocks"][1]["location"]["line"], 2);
}

#[test]
fn inspect_outputs_csv_document_json() {
    let output = Command::new(env!("CARGO_BIN_EXE_parser-cli"))
        .args([
            "inspect",
            csv_fixture_path()
                .to_str()
                .expect("CSV fixture path is UTF-8"),
        ])
        .output()
        .expect("CLI should run");

    assert!(output.status.success());
    let document: Value = serde_json::from_slice(&output.stdout).expect("stdout should be JSON");
    assert_eq!(document["source"]["source_type"], "csv");
    assert_eq!(document["source"]["delimiter"], ",");
    assert_eq!(document["blocks"][2]["location"]["row"], 2);
    assert_eq!(document["blocks"][2]["location"]["column"], 1);
}

#[test]
fn inspect_outputs_xlsx_document_json() {
    let output = Command::new(env!("CARGO_BIN_EXE_parser-cli"))
        .args([
            "inspect",
            xlsx_fixture_path()
                .to_str()
                .expect("XLSX fixture path is UTF-8"),
        ])
        .output()
        .expect("CLI should run");

    assert!(output.status.success());
    let document: Value = serde_json::from_slice(&output.stdout).expect("stdout should be JSON");
    assert_eq!(document["source"]["source_type"], "xlsx");
    assert_eq!(document["blocks"][0]["location"]["sheet"], "Data");
    assert_eq!(document["blocks"][6]["value"]["kind"], "Boolean");
}

#[test]
fn inspect_accepts_stdin() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_parser-cli"))
        .args(["inspect", "--stdin"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("CLI should run");
    child
        .stdin
        .take()
        .expect("stdin should be available")
        .write_all(b"Ada Lovelace\nGrace Hopper")
        .expect("input should be written");

    let output = child.wait_with_output().expect("CLI should finish");

    assert!(output.status.success());
    let document: Value = serde_json::from_slice(&output.stdout).expect("stdout should be JSON");
    assert_eq!(document["source"]["source_type"], "stdin");
    assert_eq!(document["blocks"].as_array().map(Vec::len), Some(2));
}

#[test]
fn inspect_accepts_pasted_text() {
    let output = Command::new(env!("CARGO_BIN_EXE_parser-cli"))
        .args(["inspect", "--text", "Ada Lovelace\nGrace Hopper"])
        .output()
        .expect("CLI should run");

    assert!(output.status.success());
    let document: Value = serde_json::from_slice(&output.stdout).expect("stdout should be JSON");
    assert_eq!(document["source"]["source_type"], "text");
    assert_eq!(document["blocks"][1]["value"]["value"], "Grace Hopper");
}

#[test]
fn inspect_reports_missing_file_as_json_error() {
    let output = Command::new(env!("CARGO_BIN_EXE_parser-cli"))
        .args(["inspect", "fixtures/text/does-not-exist.txt"])
        .output()
        .expect("CLI should run");

    assert_eq!(output.status.code(), Some(1));
    let error: Value = serde_json::from_slice(&output.stderr).expect("stderr should be JSON");
    assert_eq!(error["error"]["code"], "io_error");
    assert_eq!(error["error"]["kind"], "not_found");
}

#[test]
fn inspect_requires_command_and_input() {
    let output = Command::new(env!("CARGO_BIN_EXE_parser-cli"))
        .output()
        .expect("CLI should run");

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "usage: parser-cli inspect <path> | --stdin | --text <content> | schema validate <path>\n"
    );
}

#[test]
fn schema_validate_reports_missing_file_as_json_error() {
    let output = Command::new(env!("CARGO_BIN_EXE_parser-cli"))
        .args(["schema", "validate", "fixtures/schema/does-not-exist.json"])
        .output()
        .expect("CLI should run");

    assert_eq!(output.status.code(), Some(1));
    let error: Value = serde_json::from_slice(&output.stderr).expect("stderr should be JSON");
    assert_eq!(error["error"]["code"], "schema_io_error");
    assert!(output.stdout.is_empty());
}

#[test]
fn schema_validate_outputs_validated_schema_json() {
    let output = Command::new(env!("CARGO_BIN_EXE_parser-cli"))
        .args([
            "schema",
            "validate",
            schema_fixture_path()
                .to_str()
                .expect("schema path is UTF-8"),
        ])
        .output()
        .expect("CLI should run");

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let schema: Value = serde_json::from_slice(&output.stdout).expect("stdout should be JSON");
    assert_eq!(schema["schema_version"], "0.1");
    assert_eq!(schema["fields"][0]["name"], "email");
}

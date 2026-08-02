use serde_json::Value;
use std::{path::PathBuf, process::Command};

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/text/simple.txt")
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
fn inspect_requires_command_and_path() {
    let output = Command::new(env!("CARGO_BIN_EXE_parser-cli"))
        .output()
        .expect("CLI should run");

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "usage: parser-cli inspect <path>\n"
    );
}

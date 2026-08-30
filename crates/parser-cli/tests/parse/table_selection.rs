use super::{schema_fixture_path, support};
use serde_json::{Value, json};
use std::path::PathBuf;

fn multisheet_bytes() -> Vec<u8> {
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/xlsx/table-selection.xlsx.hex"
    ))
    .split_ascii_whitespace()
    .flat_map(|line| line.as_bytes().chunks_exact(2))
    .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
    .collect()
}

fn run(path: &std::path::Path, tail: &[&str], diagnostics: bool) -> std::process::Output {
    let schema = schema_fixture_path();
    let mut args = Vec::new();
    if diagnostics {
        args.push("--diagnostics");
    }
    args.extend([
        "parse",
        path.to_str().unwrap(),
        "--schema",
        schema.to_str().unwrap(),
    ]);
    args.extend_from_slice(tail);
    support::run(&args, None)
}

fn success(output: &std::process::Output) -> Value {
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stderr.is_empty());
    serde_json::from_slice(&output.stdout).unwrap()
}

fn usage(output: &std::process::Output) {
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(output.stdout.is_empty());
    assert_eq!(output.stderr, b"usage: parser-cli --help\n");
}

#[test]
fn csv_header_and_row_flags_are_opt_in_and_traceable() {
    let directory = support::TestDirectory::new();
    let path = directory.file(
        "rows.csv",
        b"Email,Label\n\nsecond@example.test,beta\nthird@example.test,gamma\n",
    );
    let legacy = success(&run(&path, &[], false));
    assert!(legacy["source_evidence"].get("table").is_none());
    assert_eq!(
        legacy["content"]["sheets"][0]["records"]
            .as_array()
            .unwrap()
            .len(),
        2
    );

    let selected = success(&run(
        &path,
        &[
            "--header",
            "none",
            "--include-rows",
            "1-4",
            "--exclude-rows",
            "3",
        ],
        false,
    ));
    let records = selected["content"]["sheets"][0]["records"]
        .as_array()
        .unwrap();
    assert_eq!(
        records
            .iter()
            .map(|record| record["source_row"].as_u64().unwrap())
            .collect::<Vec<_>>(),
        vec![1, 2, 4]
    );
    let rows = selected["source_evidence"]["table"]["sheets"][0]["rows"]
        .as_array()
        .unwrap();
    assert_eq!(rows[1]["blank"], true);
    assert_eq!(rows[1]["block_indices"], json!([]));
    assert_eq!(rows[2]["role"], "excluded");
    assert_eq!(
        selected["source_evidence"]["document"]["blocks"][2]["location"]["row"],
        3
    );
}

#[test]
fn sheet_selection_errors_are_processing_failures_and_diagnostics_are_opt_in() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/xlsx/sample.xlsx");
    let safe = run(&path, &["--sheet-name", "private 東京"], false);
    let report = support::error(&safe);
    assert_eq!(
        report,
        json!({
            "error": {
                "error_contract_version": "0.1",
                "code": "table_selection_error",
                "reason": "missing_sheet"
            },
            "message": "selected sheet was not found"
        })
    );
    assert!(!String::from_utf8_lossy(&safe.stderr).contains("private"));
    let detailed = support::error(&run(&path, &["--sheet-name", "private 東京"], true));
    assert_eq!(detailed["error"]["diagnostics"]["sheet"], "private 東京");
}

#[test]
fn malformed_duplicate_and_inapplicable_table_flags_are_usage_errors() {
    let directory = support::TestDirectory::new();
    let csv = directory.file("input.csv", b"email\na@example.test\n");
    let txt = directory.file("input.txt", b"a@example.test\n");
    let invalid: Vec<(&std::path::Path, Vec<&str>)> = vec![
        (&csv, vec!["--header", "row:0"]),
        (&csv, vec!["--header", "search:"]),
        (&csv, vec!["--header"]),
        (&csv, vec!["--header", "none", "--header", "auto"]),
        (&csv, vec!["--include-rows", "1,,2"]),
        (&csv, vec!["--include-rows", "1", "--include-rows", "2"]),
        (&csv, vec!["--exclude-rows", "1-"]),
        (&csv, vec!["--exclude-rows", "1", "--exclude-rows", "2"]),
        (&csv, vec!["--unknown", "value"]),
        (&csv, vec!["--sheet-index", "1"]),
        (&txt, vec!["--header", "none"]),
        (&txt, vec!["--include-rows", "1"]),
    ];
    for (path, tail) in invalid {
        usage(&run(path, &tail, false));
    }
}

#[test]
fn overlapping_ranges_and_header_conflicts_use_typed_processing_errors() {
    let directory = support::TestDirectory::new();
    let path = directory.file("input.csv", b"Email\na@example.test\n");
    for (tail, reason) in [
        (vec!["--include-rows", "1-2,2-3"], "overlapping_row_range"),
        (
            vec!["--header", "row:1", "--exclude-rows", "1"],
            "header_conflict",
        ),
    ] {
        let report = support::error(&run(&path, &tail, false));
        assert_eq!(report["error"]["code"], "table_selection_error");
        assert_eq!(report["error"]["reason"], reason);
    }
}

#[test]
fn xlsx_mixed_selectors_keep_request_order_and_empty_sheet_output() {
    let directory = support::TestDirectory::new();
    let path = directory.file("selection.xlsx", &multisheet_bytes());
    let response = success(&run(
        &path,
        &[
            "--header",
            "search:1",
            "--sheet-name",
            "Empty 東京",
            "--sheet-index",
            "3",
        ],
        false,
    ));
    let sheets = response["content"]["sheets"].as_array().unwrap();
    assert_eq!(sheets[0]["sheet"], "Empty 東京");
    assert_eq!(sheets[0]["records"], json!([]));
    assert_eq!(sheets[1]["sheet"], "Alpha");
    assert_eq!(
        response["source_evidence"]["table"]["sheets"][1]["selection_order"],
        1
    );
    assert_eq!(
        response["source_evidence"]["table"]["sheets"][2]["selection_order"],
        2
    );

    let all = success(&run(&path, &["--header", "auto"], false));
    assert_eq!(
        all["content"]["sheets"]
            .as_array()
            .unwrap()
            .iter()
            .map(|sheet| sheet["sheet"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["Alpha", "Empty 東京", "Zulu"]
    );

    let duplicate = support::error(&run(
        &path,
        &["--sheet-name", "Zulu", "--sheet-index", "1"],
        false,
    ));
    assert_eq!(duplicate["error"]["reason"], "duplicate_sheet_selection");
}

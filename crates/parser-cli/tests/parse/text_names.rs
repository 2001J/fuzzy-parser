use super::schema_compilation::{field, schema};
use super::{assert_candidate_sources, support};
use serde_json::Value;
use serde_json::json;
use std::path::PathBuf;

fn fixture(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(path)
}

#[test]
fn contact_text_header_recovers_multiword_values() {
    let input = fixture("csv/comma.csv");
    let schema = fixture("schema/contact_with_text.json");
    let output = support::run(
        &[
            "parse",
            input.to_str().unwrap(),
            "--schema",
            schema.to_str().unwrap(),
        ],
        None,
    );
    assert_eq!(output.status.code(), Some(0), "{:?}", output);
    assert!(output.stderr.is_empty());
    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    let records = response["content"]["sheets"][0]["records"]
        .as_array()
        .unwrap();
    for (record, expected) in records.iter().zip(["Ada Lovelace", "Grace Hopper"]) {
        assert_eq!(
            record["parse"]["assignment"]["fields"][0]["candidates"][0]["normalized_value"],
            expected
        );
    }
    assert_candidate_sources(&response);
}

#[test]
fn labeled_text_preserves_unicode_and_interior_whitespace() {
    let schema = fixture("schema/contact_with_text.json");
    let output = support::run(
        &["parse", "--stdin", "--schema", schema.to_str().unwrap()],
        Some("name: Zoë  東京\n".as_bytes()),
    );
    assert_eq!(output.status.code(), Some(0), "{:?}", output);
    assert!(output.stderr.is_empty());
    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    let candidate =
        &response["content"]["records"][0]["parse"]["assignment"]["fields"][0]["candidates"][0];
    assert_eq!(candidate["raw_value"], "Zoë  東京");
    assert_eq!(candidate["normalized_value"], "Zoë  東京");
    assert_eq!(candidate["confidence"], 0.8);
    assert_candidate_sources(&response);
}

fn parity(profile: &Value, path: Option<&std::path::Path>, bytes: &[u8]) -> Value {
    let directory = support::TestDirectory::new();
    let schema_path = directory.file("schema.json", profile.to_string().as_bytes());
    let args = [
        "parse",
        path.map_or("--stdin", |p| p.to_str().unwrap()),
        "--schema",
        schema_path.to_str().unwrap(),
    ];
    let output = support::run(&args, path.is_none().then_some(bytes));
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stderr.is_empty());
    let repeated = support::run(&args, path.is_none().then_some(bytes));
    assert_eq!(repeated.status.code(), Some(0));
    assert!(repeated.stderr.is_empty());
    assert_eq!(output.stdout, repeated.stdout);
    let mut detailed_args = args.to_vec();
    detailed_args.insert(0, "--diagnostics");
    let detailed = support::run(&detailed_args, path.is_none().then_some(bytes));
    assert_eq!(detailed.status.code(), Some(0));
    assert!(detailed.stderr.is_empty());
    assert_eq!(output.stdout, detailed.stdout);
    let mut reader = bytes;
    let document = match path {
        None => parser_formats::read_input(
            parser_formats::InputSource::Stdin(&mut reader),
            parser_formats::TextLimits::default(),
        )
        .unwrap(),
        Some(path) => match path.extension().unwrap().to_str().unwrap() {
            "csv" => parser_formats::read_csv(path).unwrap(),
            "xlsx" => parser_formats::read_xlsx(path).unwrap(),
            _ => parser_formats::read_txt(path).unwrap(),
        },
    };
    let plan = parser_schema::compile_schema_json(&profile.to_string()).unwrap();
    let typed = parser_schema::compile_schema(
        &parser_schema::TargetSchema::from_json(&profile.to_string()).unwrap(),
    )
    .unwrap();
    let response = parser_core::parse_document_with_plan(&document, &plan);
    assert_eq!(
        response,
        parser_core::parse_document_with_plan(&document, &typed)
    );
    assert_eq!(
        format!("{}\n", serde_json::to_string_pretty(&response).unwrap()).as_bytes(),
        output.stdout
    );
    let response = serde_json::to_value(response).unwrap();
    assert_candidate_sources(&response);
    let inspected = support::run(&["inspect", args[1]], path.is_none().then_some(bytes));
    assert_eq!(inspected.status.code(), Some(0));
    assert!(inspected.stderr.is_empty());
    assert_eq!(
        response["source_evidence"]["document"],
        serde_json::from_slice::<Value>(&inspected.stdout).unwrap()
    );
    response
}

fn records(response: &Value) -> Vec<&Value> {
    if response["content"]["mode"] == "text" {
        response["content"]["records"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| &r["parse"])
            .collect()
    } else {
        response["content"]["sheets"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|s| s["records"].as_array().unwrap().iter().map(|r| &r["parse"]))
            .collect()
    }
}

#[test]
fn txt_and_stdin_match_library_for_directed_residual_overlap_and_blank_records() {
    let directory = support::TestDirectory::new();
    let mut name = field("person", json!("person_name"));
    name["aliases"] = json!(["姓名"]);
    let profile = schema(vec![
        name,
        field("notes", json!("text")),
        field("email", json!("email")),
    ]);
    let bytes="姓名: Zoe\u{301}  東京; notes: Call later; email: ada@example.test\nAda Lovelace\nperson: Ada ada@example.test Lovelace\n\n".as_bytes();
    let path = directory.file("names.txt", bytes);
    for path in [None, Some(path.as_path())] {
        let response = parity(&profile, path, bytes);
        let records = records(&response);
        assert_eq!(records.len(), 4);
        assert_eq!(
            records[0]["assignment"]["fields"][0]["candidates"][0]["raw_value"],
            "Zoe\u{301}  東京"
        );
        assert_eq!(
            records[0]["assignment"]["fields"][1]["candidates"][0]["raw_value"],
            "Call later"
        );
        assert!(
            records[1]["assignment"]["fields"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert!(
            records[1]["assignment"]["warnings"]
                .as_array()
                .unwrap()
                .iter()
                .any(|w| w["code"] == "text_field_ambiguous")
        );
        assert_eq!(
            records[2]["assignment"]["fields"].as_array().unwrap().len(),
            1
        );
        assert_eq!(records[2]["assignment"]["fields"][0]["name"], "email");
        assert!(
            records[2]["assignment"]["warnings"]
                .as_array()
                .unwrap()
                .iter()
                .any(|w| w["code"] == "text_evidence_overlap")
        );
        assert!(records[3]["candidates"].as_array().unwrap().is_empty());
        assert!(
            records
                .iter()
                .all(|r| r["review"]["status"] == "needs_review")
        );
    }
}

#[test]
fn csv_reversed_alias_headers_and_blank_cells_match_library() {
    let directory = support::TestDirectory::new();
    let mut person = field("person", json!("person_name"));
    person["aliases"] = json!(["Name"]);
    let profile = schema(vec![person, field("notes", json!("text"))]);
    for bytes in [
        "Name,notes\nZoë  東京,Call later\n,\n",
        "notes,Name\nCall later,Zoë  東京\n,\n",
    ] {
        let path = directory.file("names.csv", bytes.as_bytes());
        let response = parity(&profile, Some(&path), bytes.as_bytes());
        let records = records(&response);
        assert_eq!(records.len(), 2);
        assert_eq!(
            records[0]["assignment"]["fields"][0]["candidates"][0]["raw_value"],
            "Zoë  東京"
        );
        assert_eq!(
            records[0]["assignment"]["fields"][1]["candidates"][0]["raw_value"],
            "Call later"
        );
        assert!(records[1]["candidates"].as_array().unwrap().is_empty());
        assert_eq!(
            records[1]["assignment"]["warnings"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
    }
}

#[test]
fn xlsx_text_headers_and_typed_cell_guards_match_library() {
    let directory = support::TestDirectory::new();
    let unicode: Vec<u8> = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/xlsx/unicode.xlsx.hex"
    ))
    .split_ascii_whitespace()
    .map(|byte| u8::from_str_radix(byte, 16).unwrap())
    .collect();
    let unicode_path = directory.file("unicode.xlsx", &unicode);
    for kind in ["text", "person_name"] {
        let profile = schema(vec![
            field("Name", json!(kind)),
            field("Count", json!(kind)),
            field("Enabled", json!(kind)),
        ]);
        for (path, expected) in [
            (fixture("xlsx/sample.xlsx"), "Ada"),
            (unicode_path.clone(), "Zoë 東京 😀"),
        ] {
            let response = parity(&profile, Some(&path), &std::fs::read(&path).unwrap());
            let records = records(&response);
            assert_eq!(records.len(), 2);
            let first = &records[0]["assignment"];
            assert_eq!(first["fields"].as_array().unwrap().len(), 1);
            assert_eq!(first["fields"][0]["name"], "Name");
            assert_eq!(first["fields"][0]["candidates"][0]["raw_value"], expected);
            assert!(
                first["warnings"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|w| w["code"] == "required_field_missing")
            );
            assert_eq!(records[1]["review"]["status"], "needs_review");
        }
    }
}

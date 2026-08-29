use super::{assert_candidate_sources, support};
use serde_json::{Value, json};
use std::{fs, path::PathBuf};

fn fixture(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(path)
}

#[test]
fn supported_profiles_preserve_full_cli_and_library_output() {
    let cases: Vec<Value> = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/contracts/schema-compilation-before.json"
    )))
    .unwrap();
    for case in cases {
        let path = fixture(case["input"].as_str().unwrap());
        let schema_path = fixture(case["profile"].as_str().unwrap());
        let bytes = fs::read(&path).unwrap();
        let stdin = case["mode"] == "stdin";
        let args = [
            "parse",
            if stdin {
                "--stdin"
            } else {
                path.to_str().unwrap()
            },
            "--schema",
            schema_path.to_str().unwrap(),
        ];
        let first = support::run(&args, stdin.then_some(bytes.as_slice()));
        assert_eq!(first.status.code(), Some(0));
        assert!(first.stderr.is_empty());
        assert_eq!(
            String::from_utf8(first.stdout.clone()).unwrap(),
            case["stdout"].as_str().unwrap()
        );
        assert_eq!(
            first.stdout,
            support::run(&args, stdin.then_some(bytes.as_slice())).stdout
        );
        let schema =
            parser_schema::TargetSchema::from_json(&fs::read_to_string(schema_path).unwrap())
                .unwrap();
        let plan = parser_schema::compile_schema(&schema).unwrap();
        let document = match case["mode"].as_str().unwrap() {
            "stdin" => parser_formats::read_input(
                parser_formats::InputSource::Stdin(&mut bytes.as_slice()),
                parser_formats::TextLimits::default(),
            )
            .unwrap(),
            "txt" => parser_formats::read_txt(&path).unwrap(),
            "csv" => parser_formats::read_csv(&path).unwrap(),
            "xlsx" => parser_formats::read_xlsx(&path).unwrap(),
            _ => unreachable!(),
        };
        let response = parser_core::parse_document_with_plan(&document, &plan);
        assert_eq!(
            format!("{}\n", serde_json::to_string_pretty(&response).unwrap()).as_bytes(),
            first.stdout
        );
        assert_candidate_sources(&serde_json::to_value(response).unwrap());
    }
}

pub(super) fn field(name: &str, kind: Value) -> Value {
    json!({"name":name,"field_type":kind,"required":true,"multiple":false,"aliases":[],"constraints":[]})
}

fn enumeration(name: &str, canonical: &str, aliases: &[&str]) -> Value {
    field(
        name,
        json!({"enum":{"values":[{"value":canonical,"aliases":aliases}]}}),
    )
}

pub(super) fn schema(fields: Vec<Value>) -> Value {
    json!({"schema_version":"0.1","record_name":"synthetic","fields":fields,"options":{"allow_unknown_fields":true}})
}

fn run_schema(schema: &Value, text: &str) -> std::process::Output {
    let directory = support::TestDirectory::new();
    let path = directory.file(
        "schema.json",
        serde_json::to_string(schema).unwrap().as_bytes(),
    );
    support::run(
        &["parse", "--stdin", "--schema", path.to_str().unwrap()],
        Some(text.as_bytes()),
    )
}

#[test]
fn disjoint_enums_do_not_assign_another_fields_value() {
    let profile = schema(vec![
        enumeration("color", "red", &[]),
        enumeration("state", "enabled", &[]),
    ]);
    let output = run_schema(&profile, "enabled");
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    let assignment = &response["content"]["records"][0]["parse"]["assignment"];
    assert_eq!(assignment["fields"][0]["name"], "state");
    assert_eq!(
        assignment["fields"][0]["candidates"][0]["normalized_value"],
        "enabled"
    );
    assert_eq!(assignment["warnings"][0]["code"], "required_field_missing");
}

#[test]
fn unsupported_options_and_constraints_fail_explicitly() {
    let mut profile = schema(vec![field("email", json!("email"))]);
    profile["options"]["allow_unknown_fields"] = json!(false);
    assert_eq!(
        support::error(&run_schema(&profile, "ada@example.test"))["error"]["code"],
        "schema_option_unsupported"
    );
    profile["options"]["allow_unknown_fields"] = json!(true);
    profile["fields"][0]["constraints"] = json!([{"kind":"minimum_integer","value":1}]);
    assert_eq!(
        support::error(&run_schema(&profile, "ada@example.test"))["error"]["code"],
        "schema_constraint_unsupported"
    );
}

#[test]
fn unknown_execution_properties_are_not_silently_discarded() {
    let mut profile = schema(vec![field("email", json!("email"))]);
    profile["options"]["locale"] = json!("private-locale");
    assert!(parser_schema::TargetSchema::from_json(&profile.to_string()).is_ok());
    assert_eq!(
        support::error(&run_schema(&profile, "ada@example.test"))["error"]["code"],
        "schema_property_unsupported"
    );
}

#[test]
fn old_schema_input_and_capability_failure_precedence_is_preserved() {
    let directory = support::TestDirectory::new();
    let missing = directory.0.join("missing.txt");
    let profile = schema(vec![field("private", json!("datetime"))]);
    let path = directory.file("schema.json", profile.to_string().as_bytes());
    let args = [
        "parse",
        missing.to_str().unwrap(),
        "--schema",
        path.to_str().unwrap(),
    ];
    assert_eq!(
        support::error(&support::run(&args, None))["error"]["code"],
        "io_error"
    );
    let path = directory.file("schema.json", b"{");
    let args = [
        "parse",
        missing.to_str().unwrap(),
        "--schema",
        path.to_str().unwrap(),
    ];
    assert_eq!(
        support::error(&support::run(&args, None))["error"]["code"],
        "schema_parse_error"
    );
}

#[test]
fn enum_cli_and_library_share_text_and_csv_ownership_and_sources() {
    let profile = schema(vec![
        enumeration("first", "active", &["go"]),
        enumeration("second", "approved", &["go"]),
    ]);
    let directory = support::TestDirectory::new();
    let schema_path = directory.file("schema.json", profile.to_string().as_bytes());
    let plan = parser_schema::compile_schema_json(&profile.to_string()).unwrap();
    for (input, csv) in [
        ("第二 second: (go), unexplained", false),
        ("go", false),
        ("first,second\n  go  ,go\n", true),
        ("second,first\n  go  ,go\n", true),
    ] {
        let path = directory.file("input.csv", input.as_bytes());
        let args = [
            "parse",
            if csv {
                path.to_str().unwrap()
            } else {
                "--stdin"
            },
            "--schema",
            schema_path.to_str().unwrap(),
        ];
        let output = support::run(&args, (!csv).then_some(input.as_bytes()));
        assert_eq!(output.status.code(), Some(0));
        assert!(output.stderr.is_empty());
        assert_eq!(
            output.stdout,
            support::run(&args, (!csv).then_some(input.as_bytes())).stdout
        );
        let document = if csv {
            parser_formats::read_csv(&path).unwrap()
        } else {
            parser_formats::read_input(
                parser_formats::InputSource::Stdin(&mut input.as_bytes()),
                parser_formats::TextLimits::default(),
            )
            .unwrap()
        };
        let response = parser_core::parse_document_with_plan(&document, &plan);
        assert_eq!(
            output.stdout,
            format!("{}\n", serde_json::to_string_pretty(&response).unwrap()).as_bytes()
        );
        let value = serde_json::to_value(response).unwrap();
        assert_candidate_sources(&value);
        let assignment = if csv {
            &value["content"]["sheets"][0]["records"][0]["parse"]["assignment"]
        } else {
            &value["content"]["records"][0]["parse"]["assignment"]
        };
        if input == "go" {
            assert_eq!(assignment["fields"], json!([]));
            assert_eq!(
                assignment["unassigned_candidates"]
                    .as_array()
                    .unwrap()
                    .len(),
                2
            );
            assert_eq!(assignment["warnings"][0]["code"], "enum_field_ambiguous");
        } else {
            for assigned in assignment["fields"].as_array().unwrap() {
                assert_eq!(
                    assigned["candidates"][0]["normalized_value"],
                    if assigned["name"] == "first" {
                        "active"
                    } else {
                        "approved"
                    }
                );
            }
            assert_eq!(
                assignment["fields"].as_array().unwrap().len(),
                if csv { 2 } else { 1 }
            );
        }
    }
}

#[test]
fn capability_errors_match_library_reports_and_keep_diagnostics_opt_in() {
    let mut option = schema(vec![]);
    option["options"]["allow_unknown_fields"] = json!(false);
    let mut constraint = schema(vec![field("private 東京\n\u{1b}", json!("email"))]);
    constraint["fields"][0]["constraints"] = json!([{"kind":"minimum_integer","value":1}]);
    let mut property = schema(vec![]);
    property["private-property"] = json!(true);
    let ambiguous = schema(vec![field(
        "private-field",
        json!({"enum":{"values":[{"value":"Active","aliases":[]},{"value":"active","aliases":[]}]}}),
    )]);
    let unsupported = schema(vec![enumeration("private-field", "in stock", &[])]);
    for profile in [option, constraint, property, ambiguous, unsupported] {
        let encoded = profile.to_string();
        let failure = parser_schema::compile_schema_json(&encoded).unwrap_err();
        let safe =
            serde_json::to_value(failure.report(parser_core::DiagnosticsMode::Safe)).unwrap();
        assert_eq!(support::error(&run_schema(&profile, "private input")), safe);
        assert!(!safe.to_string().contains("private"));
        let directory = support::TestDirectory::new();
        let path = directory.file("private-schema.json", encoded.as_bytes());
        let output = support::run(
            &[
                "--diagnostics",
                "parse",
                "--stdin",
                "--schema",
                path.to_str().unwrap(),
            ],
            Some(b"private input"),
        );
        let detailed = support::error(&output);
        let mut payload = detailed["error"].clone();
        payload.as_object_mut().unwrap().remove("diagnostics");
        assert_eq!(payload, safe["error"]);
        let expected = if failure.kind == parser_core::FailureKind::SchemaPropertyUnsupported {
            failure.with_path(path.to_str().unwrap())
        } else {
            failure
        };
        assert_eq!(
            detailed,
            serde_json::to_value(expected.report(parser_core::DiagnosticsMode::Detailed)).unwrap()
        );
        if expected.kind == parser_core::FailureKind::SchemaOptionUnsupported {
            assert!(detailed["error"].get("diagnostics").is_none());
        } else {
            assert!(
                detailed["error"]["diagnostics"]
                    .to_string()
                    .contains("private")
            );
        }
        assert!(!output.stderr.contains(&0x1b));
        assert!(!detailed.to_string().contains("private input"));
    }
}

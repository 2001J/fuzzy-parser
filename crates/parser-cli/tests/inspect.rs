use serde_json::Value;
#[path = "inspect/arguments.rs"]
mod arguments;
mod support;
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

fn invalid_schema_fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/schema/invalid.json")
}

#[test]
fn missing_absolute_input_uses_default_private_error_envelope() {
    let output = Command::new(env!("CARGO_BIN_EXE_parser-cli"))
        .args(["inspect", "/synthetic/private/東京/missing.txt"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stderr).unwrap(),
        serde_json::json!({
            "error": {"error_contract_version": "0.1", "code": "io_error", "kind": "not_found"},
            "message": "could not read input: file not found"
        })
    );
}

#[test]
fn success_output_matches_pre_error_migration_goldens() {
    let cases: Vec<Value> = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/contracts/cli-success-before-errors.json"
    )))
    .unwrap();
    for case in cases {
        for diagnostics in [false, true] {
            let mut command = Command::new(env!("CARGO_BIN_EXE_parser-cli"));
            if diagnostics {
                command.arg("--diagnostics");
            }
            let output = command
                .current_dir(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."))
                .args(
                    case["args"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|arg| arg.as_str().unwrap()),
                )
                .output()
                .unwrap();
            assert_eq!(output.status.code(), Some(0));
            assert!(output.stderr.is_empty());
            assert_eq!(
                String::from_utf8(output.stdout).unwrap(),
                case["stdout"].as_str().unwrap()
            );
        }
    }
}

#[test]
fn real_format_failures_are_redacted_by_default_and_detailed_only_on_request() {
    use serde_json::json;
    let directory = support::TestDirectory::new();
    let cases = [
        (
            directory.0.join("private missing 東京.txt"),
            json!({"code": "io_error", "kind": "not_found"}),
            "could not read input: file not found",
        ),
        (
            directory.file("private 東京.txt", b"private\xff"),
            json!({"code": "invalid_utf8", "valid_up_to": 7}),
            "input is not valid UTF-8 at byte offset 7",
        ),
        (
            directory.file("private.csv", b"name,note\nprivate,\"unclosed"),
            json!({"code": "invalid_csv", "record": null}),
            "invalid CSV input",
        ),
        (
            directory.file("private.xlsx", b"private workbook data"),
            json!({"code": "invalid_xlsx"}),
            "could not read XLSX workbook",
        ),
    ];
    for (path, mut payload, message) in cases {
        payload["error_contract_version"] = json!("0.1");
        let args = ["inspect", path.to_str().unwrap()];
        let output = support::run(&args, None);
        assert_eq!(
            support::error(&output),
            json!({"error": payload, "message": message})
        );
        assert_eq!(output.stderr, support::run(&args, None).stderr);
        assert!(!String::from_utf8_lossy(&output.stderr).contains("private"));
        payload["diagnostics"] = json!({"path": path});
        let detailed = support::error(&support::run(
            &["--diagnostics", "inspect", path.to_str().unwrap()],
            None,
        ));
        assert_eq!(detailed["error"], payload);
        assert!(!detailed.to_string().contains("unclosed"));
        assert!(!detailed.to_string().contains("workbook data"));
    }
}

#[test]
fn schema_file_and_stdin_failures_keep_codes_and_refine_invalid_data() {
    use serde_json::json;
    let directory = support::TestDirectory::new();
    let cases = [
        (directory.0.join("private missing.json"), json!({"code": "schema_io_error", "kind": "not_found"}), "could not read schema: file not found"),
        (directory.file("private encoding.json", b"private\xff"), json!({"code": "schema_io_error", "kind": "invalid_data"}), "could not read schema: invalid data"),
        (directory.file("private syntax.json", b"{private --diagnostics"), json!({"code": "schema_parse_error"}), "invalid schema JSON"),
        (directory.file("private version.json", br#"{"schema_version":"private-version","fields":[],"record_name":null,"options":{"allow_unknown_fields":true}}"#), json!({"code": "schema_validation_error", "reason": "unsupported_schema_version"}), "invalid schema: unsupported schema version"),
    ];
    for (path, mut payload, message) in cases {
        payload["error_contract_version"] = json!("0.1");
        let args = ["schema", "validate", path.to_str().unwrap()];
        let output = support::run(&args, None);
        assert_eq!(
            support::error(&output),
            json!({"error": payload, "message": message})
        );
        assert_eq!(output.stderr, support::run(&args, None).stderr);
        let mut context = json!({"path": path});
        if payload["code"] == "schema_validation_error" {
            context["version"] = json!("private-version");
        }
        payload["diagnostics"] = context;
        let detailed = support::error(&support::run(
            &[
                "--diagnostics",
                "schema",
                "validate",
                path.to_str().unwrap(),
            ],
            None,
        ));
        assert_eq!(detailed["error"], payload);
        assert!(!detailed.to_string().contains("{private"));
    }
    for detailed in [false, true] {
        let mut args = vec!["schema", "validate", "--stdin"];
        if detailed {
            args.insert(0, "--diagnostics");
        }
        assert_eq!(
            support::error(&support::run(&args, Some(b"private\xff"))),
            json!({
                "error": {"error_contract_version":"0.1", "code":"schema_io_error", "kind":"invalid_data"},
                "message": "could not read schema: invalid data"
            })
        );
    }
}

#[test]
fn diagnostics_like_input_names_content_and_trailing_arguments_do_not_opt_in() {
    let directory = support::TestDirectory::new();
    let path = directory.file("--diagnostics", b"private\xff");
    let filename_error = support::error(&support::run(&["inspect", path.to_str().unwrap()], None));
    assert!(filename_error["error"].get("diagnostics").is_none());
    let literal_name = Command::new(env!("CARGO_BIN_EXE_parser-cli"))
        .current_dir(&directory.0)
        .args(["inspect", "--diagnostics"])
        .output()
        .unwrap();
    // #6 reserves bare flag-like paths; explicit relative paths remain data.
    assert_eq!(literal_name.status.code(), Some(2));
    assert!(literal_name.stdout.is_empty());
    assert_eq!(literal_name.stderr, b"usage: parser-cli --help\n");
    let prefixed_name = Command::new(env!("CARGO_BIN_EXE_parser-cli"))
        .current_dir(&directory.0)
        .args(["inspect", "./--diagnostics"])
        .output()
        .unwrap();
    assert_eq!(support::error(&prefixed_name), filename_error);
    let schema = r#"{"schema_version":"--diagnostics","fields":[],"record_name":null,"options":{"allow_unknown_fields":true}}"#;
    for args in [
        vec!["schema", "validate", "--text", schema],
        vec!["schema", "validate", "--text", schema, "--diagnostics"],
    ] {
        let output = support::run(&args, None);
        if args.len() == 4 {
            let report = support::error(&output);
            assert!(report["error"].get("diagnostics").is_none());
        } else {
            // Exact arity rejects the formerly ignored tail before processing.
            assert_eq!(output.status.code(), Some(2));
            assert!(output.stdout.is_empty());
            assert_eq!(output.stderr, b"usage: parser-cli --help\n");
        }
        assert!(!String::from_utf8_lossy(&output.stderr).contains("--diagnostics"));
    }
    let text = support::run(&["inspect", "--text", "--diagnostics"], None);
    assert_eq!(text.status.code(), Some(0));
    assert!(text.stderr.is_empty());
    assert_eq!(
        serde_json::from_slice::<Value>(&text.stdout).unwrap()["blocks"][0]["value"]["value"],
        "--diagnostics"
    );
    let stdin = support::error(&support::run(
        &["inspect", "--stdin"],
        Some(b"--diagnostics\xff"),
    ));
    assert!(stdin["error"].get("diagnostics").is_none());
}

#[test]
fn real_text_limits_keep_numeric_metadata_without_default_source_names() {
    let directory = support::TestDirectory::new();
    for (size, mut expected, context_key) in [
        (
            65537,
            serde_json::json!({"code":"line_too_long", "line":1, "limit":65536, "actual":65537}),
            "source",
        ),
        (
            1048577,
            serde_json::json!({"code":"file_too_large", "limit":1048576, "actual":1048577}),
            "path",
        ),
    ] {
        let path = directory.file("private limit.txt", &vec![b'x'; size]);
        expected["error_contract_version"] = serde_json::json!("0.1");
        let safe = support::error(&support::run(&["inspect", path.to_str().unwrap()], None));
        assert_eq!(safe["error"], expected);
        assert!(!safe.to_string().contains("private"));
        expected["diagnostics"] = serde_json::json!({context_key:path});
        let detailed = support::error(&support::run(
            &["--diagnostics", "inspect", path.to_str().unwrap()],
            None,
        ));
        assert_eq!(detailed["error"], expected);
    }
}

#[test]
fn txt_validation_rejects_the_existing_cli_fallback_without_routing_changes() {
    let directory = support::TestDirectory::new();
    let schema = schema_fixture_path();
    for name in ["input.pdf", "input"] {
        let path = directory.file(name, b"hello");
        for args in [
            vec!["inspect", path.to_str().unwrap()],
            vec![
                "parse",
                path.to_str().unwrap(),
                "--schema",
                schema.to_str().unwrap(),
            ],
        ] {
            assert_eq!(
                support::error(&support::run(&args, None)),
                serde_json::json!({
                    "error":{"error_contract_version":"0.1","code":"unsupported_input"},
                    "message":"unsupported input type"
                })
            );
        }
    }
    // Keep the regular-file failure check on an eligible extension after #6 routing.
    let txt_directory = directory.0.join("directory.txt");
    std::fs::create_dir(&txt_directory).unwrap();
    let report = support::error(&support::run(
        &["inspect", txt_directory.to_str().unwrap()],
        None,
    ));
    assert_eq!(
        report,
        serde_json::json!({"error":{"error_contract_version":"0.1","code":"not_regular_file"},"message":"input is not a regular file"})
    );
    let detailed = support::error(&support::run(
        &["--diagnostics", "inspect", txt_directory.to_str().unwrap()],
        None,
    ));
    assert_eq!(
        detailed["error"],
        serde_json::json!({"error_contract_version":"0.1","code":"not_regular_file","diagnostics":{"path":txt_directory}})
    );
}

#[cfg(unix)]
#[test]
fn non_utf8_os_arguments_preserve_usage_versus_schema_input_distinction() {
    use std::os::unix::ffi::OsStringExt;
    for detailed in [false, true] {
        for schema in [false, true] {
            let mut command = Command::new(env!("CARGO_BIN_EXE_parser-cli"));
            if detailed {
                command.arg("--diagnostics");
            }
            if schema {
                command.args(["schema", "validate"]);
            } else {
                command.arg("inspect");
            }
            let output = command
                .arg("--text")
                .arg(std::ffi::OsString::from_vec(b"private\xff".to_vec()))
                .output()
                .unwrap();
            assert!(output.stdout.is_empty());
            if schema {
                assert_eq!(
                    support::error(&output),
                    serde_json::json!({
                        "error": {"error_contract_version":"0.1", "code":"schema_input_error"},
                        "message": "schema text must be valid UTF-8"
                    })
                );
            } else {
                assert_eq!(output.status.code(), Some(2));
                assert_eq!(output.stderr, b"text argument must be valid UTF-8\n");
            }
        }
    }
}

#[cfg(unix)]
#[test]
fn unreadable_files_report_permission_denied_from_a_non_root_process() {
    use std::os::unix::{fs::PermissionsExt, process::CommandExt};
    let directory = support::TestDirectory::new();
    std::fs::set_permissions(&directory.0, std::fs::Permissions::from_mode(0o755)).unwrap();
    let path = directory.file("private denied.txt", b"private record");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).unwrap();
    let uid = Command::new("id").arg("-u").output().unwrap();
    assert!(uid.status.success());
    let root = String::from_utf8(uid.stdout).unwrap().trim() == "0";
    for schema in [false, true] {
        for detailed in [false, true] {
            let mut command = Command::new(env!("CARGO_BIN_EXE_parser-cli"));
            if root {
                command.gid(65534).uid(65534);
            }
            if detailed {
                command.arg("--diagnostics");
            }
            if schema {
                command.args(["schema", "validate"]);
            } else {
                command.arg("inspect");
            }
            let output = command.arg(&path).output().unwrap();
            let mut payload = serde_json::json!({"error_contract_version":"0.1", "code": if schema {"schema_io_error"} else {"io_error"}, "kind":"permission_denied"});
            if detailed {
                payload["diagnostics"] = serde_json::json!({"path": path});
            }
            assert_eq!(support::error(&output)["error"], payload);
        }
    }
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
fn xlsx_inspect_matches_file_and_byte_serialization() {
    let bytes = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/xlsx/sample.xlsx"
    ));
    let from_bytes = parser_formats::read_xlsx_bytes(Some("sample.xlsx"), bytes).unwrap();
    let from_file = parser_formats::read_xlsx(xlsx_fixture_path()).unwrap();
    let json = serde_json::to_string_pretty(&from_bytes).unwrap();
    assert_eq!(json, serde_json::to_string_pretty(&from_file).unwrap());
    let output = Command::new(env!("CARGO_BIN_EXE_parser-cli"))
        .arg("inspect")
        .arg(xlsx_fixture_path())
        .output()
        .expect("CLI should run");
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    assert_eq!(output.stdout, format!("{json}\n").as_bytes());

    let unnamed = parser_formats::read_xlsx_bytes(None, bytes).unwrap();
    let unnamed_json = serde_json::to_value(&unnamed).unwrap();
    assert!(unnamed_json["source"]["file_name"].is_null());
    let mut expected: Value = serde_json::from_str(&json).unwrap();
    expected["source"]["file_name"] = Value::Null;
    assert_eq!(unnamed_json, expected);
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
        "usage: parser-cli --help\n"
    );
}

#[test]
fn help_lists_schema_validation_modes() {
    let output = Command::new(env!("CARGO_BIN_EXE_parser-cli"))
        .arg("--help")
        .output()
        .expect("CLI should run");

    assert!(output.status.success());
    let help = String::from_utf8_lossy(&output.stdout);
    assert!(help.contains("schema validate <path>"));
    assert!(help.contains("schema validate --stdin"));
    assert!(help.contains("schema validate --text <content>"));
}

#[test]
fn schema_help_is_available_from_the_subcommand() {
    let output = Command::new(env!("CARGO_BIN_EXE_parser-cli"))
        .args(["schema", "validate", "--help"])
        .output()
        .expect("CLI should run");

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("schema validate"));
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

#[test]
fn schema_validate_compact_outputs_one_json_line() {
    let output = Command::new(env!("CARGO_BIN_EXE_parser-cli"))
        .args([
            "schema",
            "validate",
            "--compact",
            schema_fixture_path()
                .to_str()
                .expect("schema path is UTF-8"),
        ])
        .output()
        .expect("CLI should run");

    assert!(output.status.success());
    assert_eq!(
        output.stdout.iter().filter(|byte| **byte == b'\n').count(),
        1
    );
    let schema: Value = serde_json::from_slice(&output.stdout).expect("stdout should be JSON");
    assert_eq!(schema["record_name"], "contact");
}

#[test]
fn schema_validate_accepts_stdin() {
    let schema =
        std::fs::read_to_string(schema_fixture_path()).expect("schema fixture should read");
    let mut child = Command::new(env!("CARGO_BIN_EXE_parser-cli"))
        .args(["schema", "validate", "--stdin"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("CLI should run");
    child
        .stdin
        .take()
        .expect("stdin should be available")
        .write_all(schema.as_bytes())
        .expect("schema should be written");

    let output = child.wait_with_output().expect("CLI should finish");

    assert!(output.status.success());
    let result: Value = serde_json::from_slice(&output.stdout).expect("stdout should be JSON");
    assert_eq!(result["record_name"], "contact");
}

#[test]
fn schema_validate_accepts_inline_text() {
    let schema = r#"{"schema_version":"0.1","record_name":"inline","fields":[],"options":{"allow_unknown_fields":true}}"#;
    let output = Command::new(env!("CARGO_BIN_EXE_parser-cli"))
        .args(["schema", "validate", "--text", schema])
        .output()
        .expect("CLI should run");

    assert!(output.status.success());
    let result: Value = serde_json::from_slice(&output.stdout).expect("stdout should be JSON");
    assert_eq!(result["record_name"], "inline");
}

#[test]
fn schema_validate_reports_invalid_schema_as_json_error() {
    let output = Command::new(env!("CARGO_BIN_EXE_parser-cli"))
        .args([
            "schema",
            "validate",
            invalid_schema_fixture_path()
                .to_str()
                .expect("schema path is UTF-8"),
        ])
        .output()
        .expect("CLI should run");

    assert_eq!(output.status.code(), Some(1));
    let error: Value = serde_json::from_slice(&output.stderr).expect("stderr should be JSON");
    assert_eq!(error["error"]["code"], "schema_validation_error");
    assert!(
        error["message"]
            .as_str()
            .is_some_and(|message| message.contains("field name"))
    );
}

#[test]
fn schema_validate_reports_malformed_json_as_json_error() {
    let output = Command::new(env!("CARGO_BIN_EXE_parser-cli"))
        .args(["schema", "validate", "--text", "{malformed"])
        .output()
        .expect("CLI should run");

    assert_eq!(output.status.code(), Some(1));
    let error: Value = serde_json::from_slice(&output.stderr).expect("stderr should be JSON");
    assert_eq!(error["error"]["code"], "schema_parse_error");
    assert!(
        error["message"]
            .as_str()
            .is_some_and(|message| message.contains("invalid schema JSON"))
    );
}

use super::{csv_fixture_path, fixture_path, schema_fixture_path, support, xlsx_fixture_path};
use serde_json::{Value, json};
use std::{fs, path::Path, process::Command};

fn run(args: &[String], input: Option<&[u8]>) -> std::process::Output {
    support::run(&args.iter().map(String::as_str).collect::<Vec<_>>(), input)
}

fn usage(output: &std::process::Output) {
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(output.stdout.is_empty());
    assert_eq!(output.stderr, b"usage: parser-cli --help\n");
}

fn success(output: &std::process::Output) -> Value {
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stderr.is_empty());
    serde_json::from_slice(&output.stdout).unwrap()
}

fn file_args(path: &Path, parse: bool, options: &[&str]) -> Vec<String> {
    let mut args = vec![
        if parse { "parse" } else { "inspect" }.into(),
        path.to_str().unwrap().into(),
    ];
    if parse {
        args.extend([
            "--schema".into(),
            schema_fixture_path().to_str().unwrap().into(),
        ]);
    }
    args.extend(options.iter().map(|value| (*value).to_owned()));
    args
}

#[test]
fn help_is_exact_at_every_command_level() {
    for prefix in [
        vec![],
        vec!["inspect"],
        vec!["parse"],
        vec!["schema"],
        vec!["schema", "validate"],
    ] {
        for flag in ["-h", "--help"] {
            for diagnostics in [false, true] {
                let mut args = Vec::new();
                if diagnostics {
                    args.push("--diagnostics");
                }
                args.extend(prefix.iter().copied());
                args.push(flag);
                let output = support::run(&args, None);
                assert_eq!(output.status.code(), Some(0), "{args:?}: {output:?}");
                assert!(output.stderr.is_empty());
                let help = String::from_utf8(output.stdout).unwrap();
                for required in [
                    "inspect <path>",
                    "inspect --stdin",
                    "inspect --text <content>",
                    "parse <path> --schema <schema-path>",
                    "parse --stdin --schema <schema-path>",
                    "schema validate <path>",
                    "schema validate --stdin",
                    "schema validate --text <content>",
                    "schema validate --compact <path>",
                    "--diagnostics",
                    "--max-bytes",
                    "--empty",
                    "1048576",
                    "65536",
                    "TXT",
                    "accept",
                    "reject",
                ] {
                    assert!(help.contains(required), "missing {required}: {help}");
                }
                for extra in ["private-extra", "--diagnostics", "--help"] {
                    let mut invalid = args.clone();
                    invalid.push(extra);
                    usage(&support::run(&invalid, None));
                }
            }
        }
    }
}

#[test]
fn nine_forms_preserve_success_and_reject_every_extra_tail() {
    let txt = fixture_path();
    let schema = schema_fixture_path();
    let schema_text = fs::read_to_string(&schema).unwrap();
    let cases = [
        (vec!["inspect", txt.to_str().unwrap()], None),
        (
            vec!["inspect", "--stdin"],
            Some(b"ada@example.test".as_slice()),
        ),
        (vec!["inspect", "--text", "--diagnostics"], None),
        (
            vec![
                "parse",
                txt.to_str().unwrap(),
                "--schema",
                schema.to_str().unwrap(),
            ],
            None,
        ),
        (
            vec!["parse", "--stdin", "--schema", schema.to_str().unwrap()],
            Some(b"ada@example.test".as_slice()),
        ),
        (vec!["schema", "validate", schema.to_str().unwrap()], None),
        (
            vec!["schema", "validate", "--stdin"],
            Some(schema_text.as_bytes()),
        ),
        (vec!["schema", "validate", "--text", &schema_text], None),
        (
            vec!["schema", "validate", "--compact", schema.to_str().unwrap()],
            None,
        ),
    ];
    for (args, input) in cases {
        let ordinary = support::run(&args, input);
        success(&ordinary);
        let mut detailed = vec!["--diagnostics"];
        detailed.extend(args.iter().copied());
        let output = support::run(&detailed, input);
        success(&output);
        assert_eq!(ordinary.stdout, output.stdout);
        for base in [&args, &detailed] {
            for tail in ["private-extra", "--diagnostics", "--help", "--schema"] {
                let mut invalid = base.clone();
                invalid.push(tail);
                usage(&support::run(&invalid, None));
            }
        }
    }
}

#[test]
fn missing_duplicate_misplaced_and_unknown_arguments_are_private_usage_errors() {
    let cases: &[&[&str]] = &[
        &[],
        &["inspect"],
        &["parse"],
        &["schema"],
        &["schema", "validate"],
        &["--diagnostics"],
        &["--diagnostics", "--diagnostics", "inspect", "private.txt"],
        &["private-command"],
        &["schema", "private-action"],
        &["inspect", "--private"],
        &["inspect", "--diagnostics"],
        &["inspect", "-private.txt"],
        &["inspect", "--", "private.txt"],
        &["inspect", "--text"],
        &["inspect", "--stdin", "--stdin"],
        &["inspect", "--max-bytes", "1", "private.txt"],
        &["inspect", "private.txt", "--max-bytes"],
        &["inspect", "private.txt", "--empty"],
        &["inspect", "private.txt", "--private", "value"],
        &["inspect", "private.txt", "--max-bytes=1"],
        &["inspect", "private.txt", "--empty=accept"],
        &[
            "inspect",
            "private.txt",
            "--max-bytes",
            "1",
            "--max-bytes",
            "1",
        ],
        &[
            "inspect",
            "private.txt",
            "--empty",
            "accept",
            "--empty",
            "reject",
        ],
        &[
            "inspect",
            "private.txt",
            "--empty",
            "accept",
            "--max-bytes",
            "2",
            "--empty",
            "accept",
        ],
        &["inspect", "--stdin", "--max-bytes", "1"],
        &["inspect", "--stdin", "--empty", "accept"],
        &["inspect", "--text", "private", "--max-bytes", "1"],
        &["inspect", "--text", "private", "--empty", "reject"],
        &["parse", "private.txt"],
        &["parse", "private.txt", "--schema"],
        &["parse", "--schema", "private.json", "private.txt"],
        &[
            "parse",
            "--input",
            "private.txt",
            "--schema",
            "private.json",
        ],
        &["parse", "--text", "private", "--schema", "private.json"],
        &["parse", "private.txt", "--schema", "--private.json"],
        &[
            "parse",
            "private.txt",
            "--max-bytes",
            "1",
            "--schema",
            "private.json",
        ],
        &[
            "parse",
            "private.txt",
            "--schema",
            "private.json",
            "--schema",
            "private.json",
        ],
        &[
            "parse",
            "--stdin",
            "--schema",
            "private.json",
            "--empty",
            "accept",
        ],
        &[
            "parse",
            "--stdin",
            "--schema",
            "private.json",
            "--max-bytes",
            "1",
        ],
        &["schema", "validate", "--text"],
        &["schema", "validate", "--compact"],
        &["schema", "validate", "--compact", "--stdin"],
        &["schema", "validate", "--compact", "--text", "private"],
        &[
            "schema",
            "validate",
            "--compact",
            "private.json",
            "--compact",
        ],
        &["schema", "validate", "private.json", "--empty", "accept"],
        &["schema", "validate", "--stdin", "--max-bytes", "1"],
        &[
            "schema", "validate", "--text", "private", "--empty", "reject",
        ],
        &["schema", "validate", "--private.json"],
    ];
    for args in cases {
        usage(&support::run(args, None));
        let mut detailed = vec!["--diagnostics"];
        detailed.extend_from_slice(args);
        usage(&support::run(&detailed, None));
    }
}

#[test]
fn option_values_are_strict_before_any_io_or_extension_failure() {
    for parse in [false, true] {
        for extension in ["txt", "csv", "xlsx", "unknown"] {
            let path = std::path::PathBuf::from(format!("private-missing.{extension}"));
            for bad in [
                "",
                "+1",
                "-1",
                " 1",
                "1 ",
                "1.0",
                "1KiB",
                "１",
                "184467440737095516160",
                "--diagnostics",
            ] {
                usage(&run(&file_args(&path, parse, &["--max-bytes", bad]), None));
            }
            for bad in ["", "Accept", "REJECT", "true", "--diagnostics"] {
                usage(&run(&file_args(&path, parse, &["--empty", bad]), None));
            }
        }
        for extension in ["csv", "CSV", "xlsx", "XLSX"] {
            let path = std::path::PathBuf::from(format!("private-missing.{extension}"));
            for options in [vec!["--empty", "accept"], vec!["--max-bytes", "1048576"]] {
                usage(&run(&file_args(&path, parse, &options), None));
            }
        }
    }
}

#[test]
fn txt_empty_whitespace_and_size_options_reuse_the_library_contract() {
    let directory = support::TestDirectory::new();
    for parse in [false, true] {
        let maximum = usize::MAX.to_string();
        success(&run(
            &file_args(&fixture_path(), parse, &["--max-bytes", &maximum]),
            None,
        ));
        for (name, bytes) in [
            ("empty.txt", b"".as_slice()),
            ("space.txt", b" \t\n"),
            ("exact.TXT", b"abc"),
        ] {
            let path = directory.file(name, bytes);
            let baseline = run(&file_args(&path, parse, &[]), None);
            success(&baseline);
            for options in [
                vec!["--empty", "accept"],
                vec!["--max-bytes", "1048576"],
                vec!["--max-bytes", "0003", "--empty", "accept"],
                vec!["--empty", "accept", "--max-bytes", "3"],
            ] {
                let output = run(&file_args(&path, parse, &options), None);
                success(&output);
                assert_eq!(output.stdout, baseline.stdout);
            }
            let rejected = run(&file_args(&path, parse, &["--empty", "reject"]), None);
            if bytes.is_empty() {
                assert_eq!(support::error(&rejected)["error"]["code"], "empty_input");
                success(&run(&file_args(&path, parse, &["--max-bytes", "0"]), None));
                assert_eq!(
                    support::error(&run(
                        &file_args(&path, parse, &["--empty", "reject", "--max-bytes", "0"]),
                        None
                    ))["error"]["code"],
                    "empty_input"
                );
            } else {
                assert_eq!(success(&rejected), success(&baseline));
                for limit in ["0", "2"] {
                    let error = support::error(&run(
                        &file_args(&path, parse, &["--max-bytes", limit]),
                        None,
                    ));
                    assert_eq!(
                        error["error"],
                        json!({"error_contract_version":"0.1", "code":"file_too_large", "limit":limit.parse::<u64>().unwrap(), "actual":3})
                    );
                }
            }
        }
        let invalid = directory.file("invalid.txt", b"abc\xff");
        assert_eq!(
            support::error(&run(
                &file_args(&invalid, parse, &["--max-bytes", "3"]),
                None
            ))["error"]["code"],
            "file_too_large"
        );
        assert_eq!(
            support::error(&run(
                &file_args(&invalid, parse, &["--max-bytes", "4"]),
                None
            ))["error"]["code"],
            "invalid_utf8"
        );
    }
}

#[test]
fn raised_txt_byte_limit_does_not_change_line_or_schema_limits() {
    let directory = support::TestDirectory::new();
    let mut bytes = vec![b'a'; 1048578];
    for index in (65535..bytes.len()).step_by(65536) {
        bytes[index] = b'\n';
    }
    let large = directory.file("large.txt", &bytes);
    for parse in [false, true] {
        assert_eq!(
            support::error(&run(&file_args(&large, parse, &[]), None))["error"]["code"],
            "file_too_large"
        );
    }
    for parse in [false, true] {
        let output = run(&file_args(&large, parse, &["--max-bytes", "1048578"]), None);
        let document = success(&output);
        let source = if parse {
            &document["source_evidence"]["document"]["source"]
        } else {
            &document["source"]
        };
        assert_eq!(source["size_bytes"], 1048578);
    }
    let line = directory.file("line.txt", &vec![b'a'; 65537]);
    for parse in [false, true] {
        assert_eq!(
            support::error(&run(
                &file_args(&line, parse, &["--max-bytes", "9999999"]),
                None
            ))["error"]["code"],
            "line_too_long"
        );
    }
}

#[test]
fn extension_dispatch_is_explicit_before_filesystem_access() {
    let directory = support::TestDirectory::new();
    for name in ["missing", "missing.pdf", "missing.txt.pdf"] {
        let path = directory.0.join(name);
        for parse in [false, true] {
            for options in [vec![], vec!["--max-bytes", "0", "--empty", "reject"]] {
                let output = run(&file_args(&path, parse, &options), None);
                assert_eq!(
                    support::error(&output)["error"],
                    json!({"error_contract_version":"0.1", "code":"unsupported_input"})
                );
            }
        }
        fs::create_dir(&path).unwrap();
        assert_eq!(
            support::error(&run(&file_args(&path, false, &[]), None))["error"]["code"],
            "unsupported_input"
        );
    }
    let path = directory.0.join("directory.txt");
    fs::create_dir(&path).unwrap();
    assert_eq!(
        support::error(&run(&file_args(&path, false, &[]), None))["error"]["code"],
        "not_regular_file"
    );
    assert_eq!(
        support::error(&run(
            &file_args(&directory.0.join("missing.txt"), false, &[]),
            None
        ))["error"]["code"],
        "io_error"
    );
}

#[test]
fn known_uppercase_formats_keep_adapters_and_csv_xlsx_have_no_txt_byte_cap() {
    let directory = support::TestDirectory::new();
    for (name, fixture, source) in [
        ("TEXT.TXT", fixture_path(), "txt"),
        ("TABLE.CsV", csv_fixture_path(), "csv"),
        ("BOOK.XlSx", xlsx_fixture_path(), "xlsx"),
    ] {
        let path = directory.file(name, &fs::read(fixture).unwrap());
        for parse in [false, true] {
            let document = success(&run(&file_args(&path, parse, &[]), None));
            let source_metadata = if parse {
                &document["source_evidence"]["document"]["source"]
            } else {
                &document["source"]
            };
            assert_eq!(source_metadata["source_type"], source);
            assert_eq!(source_metadata["file_name"], name);
        }
    }
    let csv = directory.file("large.csv", &vec![b'a'; 1048577]);
    assert_eq!(
        success(&run(&file_args(&csv, false, &[]), None))["source"]["size_bytes"],
        1048577
    );
    let xlsx = directory.file("large.xlsx", &vec![b'a'; 1048577]);
    assert_eq!(
        support::error(&run(&file_args(&xlsx, false, &[]), None))["error"]["code"],
        "invalid_xlsx"
    );
}

#[test]
fn parse_keeps_strict_decode_then_extraction_then_compilation_precedence() {
    let directory = support::TestDirectory::new();
    let input = directory.file("input.txt", b"abc");
    let mut schema: Value =
        serde_json::from_slice(&fs::read(schema_fixture_path()).unwrap()).unwrap();
    schema["options"]["allow_unknown_fields"] = json!(false);
    let compile_failure = directory.file(
        "compile.json",
        serde_json::to_string(&schema).unwrap().as_bytes(),
    );
    schema["private-unknown"] = json!(true);
    let decode_failure = directory.file(
        "decode.json",
        serde_json::to_string(&schema).unwrap().as_bytes(),
    );
    for (schema, code) in [
        (&decode_failure, "schema_property_unsupported"),
        (&compile_failure, "unsupported_input"),
    ] {
        let missing = directory.0.join("missing.pdf");
        assert_eq!(
            support::error(&support::run(
                &[
                    "parse",
                    missing.to_str().unwrap(),
                    "--schema",
                    schema.to_str().unwrap(),
                    "--empty",
                    "reject"
                ],
                None
            ))["error"]["code"],
            code
        );
    }
    for (schema, code) in [
        (&decode_failure, "schema_property_unsupported"),
        (&compile_failure, "file_too_large"),
    ] {
        let args = [
            "parse",
            input.to_str().unwrap(),
            "--schema",
            schema.to_str().unwrap(),
            "--max-bytes",
            "2",
        ];
        assert_eq!(
            support::error(&support::run(&args, None))["error"]["code"],
            code
        );
        let mut extra = args.to_vec();
        extra.push("private-extra");
        usage(&support::run(&extra, None));
    }
    assert_eq!(
        support::error(&support::run(
            &[
                "parse",
                input.to_str().unwrap(),
                "--schema",
                compile_failure.to_str().unwrap(),
                "--max-bytes",
                "3"
            ],
            None
        ))["error"]["code"],
        "schema_option_unsupported"
    );
}

#[test]
fn diagnostics_is_leading_only_and_literal_text_and_prefixed_paths_stay_data() {
    let directory = support::TestDirectory::new();
    let path = directory.file("--diagnostics.txt", b"\xff");
    for name in ["./--diagnostics.txt", path.to_str().unwrap()] {
        let output = Command::new(env!("CARGO_BIN_EXE_parser-cli"))
            .current_dir(&directory.0)
            .args(["inspect", name])
            .output()
            .unwrap();
        assert!(
            support::error(&output)["error"]
                .get("diagnostics")
                .is_none()
        );
    }
    for literal in ["--diagnostics", "--help", "-h", "--max-bytes", ""] {
        let output = support::run(&["inspect", "--text", literal], None);
        let document = success(&output);
        if !literal.is_empty() {
            assert_eq!(document["blocks"][0]["value"]["value"], literal);
        }
        assert_eq!(
            support::error(&support::run(
                &["schema", "validate", "--text", literal],
                None
            ))["error"]["code"],
            "schema_parse_error"
        );
    }
    for parse in [false, true] {
        let options = ["--max-bytes", "0"];
        let safe = run(&file_args(&path, parse, &options), None);
        assert!(support::error(&safe)["error"].get("diagnostics").is_none());
        let mut args = vec!["--diagnostics".to_owned()];
        args.extend(file_args(&path, parse, &options));
        let detailed = support::error(&run(&args, None));
        assert_eq!(detailed["error"]["diagnostics"], json!({"path":path}));
        assert_eq!(detailed["error"]["code"], "file_too_large");
    }
}

#[cfg(unix)]
#[test]
fn non_utf8_argv_preserves_paths_but_rejects_syntax_and_option_values() {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};
    let invalid = OsString::from_vec(b"private\xff".to_vec());
    for mut args in [
        vec![invalid.clone()],
        vec![
            "inspect".into(),
            "private.txt".into(),
            "--max-bytes".into(),
            invalid.clone(),
        ],
        vec![
            "inspect".into(),
            "private.txt".into(),
            "--empty".into(),
            invalid.clone(),
        ],
        vec![
            "parse".into(),
            "private.txt".into(),
            invalid.clone(),
            "schema.json".into(),
        ],
        vec![
            "schema".into(),
            "validate".into(),
            "--text".into(),
            invalid.clone(),
            "extra".into(),
        ],
    ] {
        for detailed in [false, true] {
            if detailed {
                args.insert(0, "--diagnostics".into());
            }
            usage(
                &Command::new(env!("CARGO_BIN_EXE_parser-cli"))
                    .args(&args)
                    .output()
                    .unwrap(),
            );
        }
    }
    let directory = support::TestDirectory::new();
    let schema_path = directory
        .0
        .join(OsString::from_vec(b"schema\xff.json".to_vec()));
    for args in [
        vec![
            OsString::from("schema"),
            "validate".into(),
            schema_path.as_os_str().to_owned(),
        ],
        vec![
            "parse".into(),
            fixture_path().into_os_string(),
            "--schema".into(),
            schema_path.into_os_string(),
        ],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_parser-cli"))
            .args(args)
            .output()
            .unwrap();
        assert_eq!(support::error(&output)["error"]["code"], "schema_io_error");
    }
    for (name, code) in [
        (b"private\xff.txt".as_slice(), "io_error"),
        (b"private.\xff", "unsupported_input"),
    ] {
        let path = directory.0.join(OsString::from_vec(name.to_vec()));
        let output = Command::new(env!("CARGO_BIN_EXE_parser-cli"))
            .arg("inspect")
            .arg(&path)
            .output()
            .unwrap();
        assert_eq!(support::error(&output)["error"]["code"], code);
    }
}

#[test]
fn text_and_stdin_empty_acceptance_and_fixed_limits_are_unchanged() {
    for args in [vec!["inspect", "--text", ""], vec!["inspect", "--stdin"]] {
        assert_eq!(success(&support::run(&args, None))["blocks"], json!([]));
    }
    let schema = schema_fixture_path();
    let parsed = support::run(
        &["parse", "--stdin", "--schema", schema.to_str().unwrap()],
        None,
    );
    assert_eq!(
        success(&parsed)["source_evidence"]["document"]["blocks"],
        json!([])
    );
    let bytes = vec![b'a'; 1048577];
    assert_eq!(
        support::error(&support::run(&["inspect", "--stdin"], Some(&bytes)))["error"]["code"],
        "input_too_large"
    );
}

use super::{assert_candidate_sources, support};
use parser_core::{CandidateType, ParseContent, ParseResponse, RawDocument, TextParseResult};
use serde_json::{Value, json};
use std::{fs, path::PathBuf};

const PROFILE_RECORD_NAMES: [&str; 2] = ["synthetic_attendance_entry", "synthetic_inventory_entry"];

#[derive(Clone, Copy)]
struct ProfileCase {
    fixture: &'static str,
    first_text_fields: &'static [&'static str],
    first_xlsx_fields: &'static [&'static str],
}

const PROFILES: [ProfileCase; 2] = [
    ProfileCase {
        fixture: "conformance/attendance-profile.json",
        first_text_fields: &[
            "participant",
            "contact_email",
            "party_size",
            "attending",
            "note",
        ],
        first_xlsx_fields: &["participant", "attending"],
    },
    ProfileCase {
        fixture: "conformance/inventory-profile.json",
        first_text_fields: &[
            "item_label",
            "supplier_email",
            "units",
            "available",
            "stock_state",
            "handling_note",
        ],
        first_xlsx_fields: &["item_label", "units", "available"],
    },
];

#[derive(Clone, Copy)]
enum InputCase {
    Pasted,
    Txt,
    Csv,
    Xlsx,
}

const INPUTS: [InputCase; 4] = [
    InputCase::Pasted,
    InputCase::Txt,
    InputCase::Csv,
    InputCase::Xlsx,
];

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fixture(path: &str) -> PathBuf {
    repository_root().join("fixtures").join(path)
}

fn input_path(input: InputCase) -> PathBuf {
    fixture(match input {
        InputCase::Pasted | InputCase::Txt => "conformance/shared.txt",
        InputCase::Csv => "conformance/shared.csv",
        InputCase::Xlsx => "xlsx/sample.xlsx",
    })
}

fn native_document(input: InputCase, path: &std::path::Path, bytes: &[u8]) -> RawDocument {
    match input {
        InputCase::Pasted => {
            let mut reader = bytes;
            parser_formats::read_input(
                parser_formats::InputSource::Stdin(&mut reader),
                parser_formats::TextLimits::default(),
            )
            .unwrap()
        }
        InputCase::Txt => parser_formats::read_txt(path).unwrap(),
        InputCase::Csv => parser_formats::read_csv(path).unwrap(),
        InputCase::Xlsx => parser_formats::read_xlsx(path).unwrap(),
    }
}

fn parses(response: &ParseResponse) -> Vec<&parser_core::TextParseResult> {
    match &response.content {
        ParseContent::Text { records } => records.iter().map(|record| &record.parse).collect(),
        ParseContent::Table { sheets } => sheets
            .iter()
            .flat_map(|sheet| sheet.records.iter().map(|record| &record.parse))
            .collect(),
    }
}

fn has_unused_source(response: &ParseResponse) -> bool {
    let evidence = response.source_evidence.as_ref().unwrap();
    evidence
        .blocks
        .iter()
        .flat_map(|coverage| &coverage.unused_spans)
        .any(|span| span.byte_start < span.byte_end)
}

fn assert_assignment(
    parse: &TextParseResult,
    name: &str,
    candidate_type: CandidateType,
    raw: &str,
    normalized: Value,
) {
    let field = parse
        .assignment
        .fields
        .iter()
        .find(|field| field.name == name)
        .unwrap_or_else(|| panic!("missing expected field {name}"));
    assert_eq!(field.candidates.len(), 1);
    let candidate = &field.candidates[0];
    assert_eq!(candidate.candidate_type, candidate_type);
    assert_eq!(candidate.raw_value, raw);
    assert_eq!(candidate.normalized_value.as_ref(), Some(&normalized));
}

fn assert_first_record_semantics(profile: ProfileCase, input: InputCase, parse: &TextParseResult) {
    let table = matches!(input, InputCase::Xlsx);
    let name = if table { "Ada" } else { "Zoë  東京" };
    let count = if table { 42 } else { 3 };
    let note = if matches!(input, InputCase::Csv) {
        "Keep  dry"
    } else {
        "Keep  dry; Spare: untouched"
    };

    if profile.fixture.contains("attendance") {
        assert_assignment(
            parse,
            "participant",
            CandidateType::PersonName,
            name,
            json!(name),
        );
        assert_assignment(
            parse,
            "attending",
            CandidateType::Boolean,
            "true",
            json!(true),
        );
        if table {
            assert!(
                parse
                    .assignment
                    .unassigned_candidates
                    .iter()
                    .any(|candidate| {
                        candidate.candidate_type == CandidateType::Integer
                            && candidate.normalized_value == Some(json!(42))
                    })
            );
        } else {
            assert_assignment(
                parse,
                "contact_email",
                CandidateType::Email,
                "zoe@example.test",
                json!("zoe@example.test"),
            );
            assert_assignment(parse, "party_size", CandidateType::Integer, "3", json!(3));
            assert_assignment(parse, "note", CandidateType::Text, note, json!(note));
        }
    } else {
        assert_assignment(parse, "item_label", CandidateType::Text, name, json!(name));
        assert_assignment(
            parse,
            "units",
            CandidateType::Integer,
            &count.to_string(),
            json!(count),
        );
        assert_assignment(
            parse,
            "available",
            CandidateType::Boolean,
            "true",
            json!(true),
        );
        if !table {
            assert_assignment(
                parse,
                "supplier_email",
                CandidateType::Email,
                "zoe@example.test",
                json!("zoe@example.test"),
            );
            assert_assignment(
                parse,
                "stock_state",
                CandidateType::Enum,
                "in",
                json!("available"),
            );
            assert_assignment(
                parse,
                "handling_note",
                CandidateType::Text,
                note,
                json!(note),
            );
        }
    }
}

#[test]
fn two_profiles_share_native_and_cli_semantics_across_supported_formats() {
    for profile in PROFILES {
        let schema_path = fixture(profile.fixture);
        let schema_json = fs::read_to_string(&schema_path).unwrap();
        let schema = parser_schema::TargetSchema::from_json(&schema_json).unwrap();
        let plan = parser_schema::compile_schema(&schema).unwrap();
        let mut saw_unused_source = false;

        for input in INPUTS {
            let path = input_path(input);
            let bytes = fs::read(&path).unwrap();
            let input_arg = if matches!(input, InputCase::Pasted) {
                "--stdin"
            } else {
                path.to_str().unwrap()
            };
            let args = [
                "parse",
                input_arg,
                "--schema",
                schema_path.to_str().unwrap(),
            ];
            let stdin = matches!(input, InputCase::Pasted).then_some(bytes.as_slice());
            let first = support::run(&args, stdin);
            let second = support::run(&args, stdin);
            assert_eq!(first.status.code(), Some(0), "{first:?}");
            assert!(first.stderr.is_empty());
            assert_eq!(first.stdout, second.stdout);
            assert_eq!(first.stderr, second.stderr);

            let document = native_document(input, &path, &bytes);
            let native = parser_core::parse_document_with_plan(&document, &plan);
            assert_eq!(
                native,
                parser_core::parse_document_with_plan(&document, &plan)
            );
            assert_eq!(
                first.stdout,
                format!("{}\n", serde_json::to_string_pretty(&native).unwrap()).as_bytes()
            );
            let serialized = serde_json::to_value(&native).unwrap();
            assert_candidate_sources(&serialized);
            saw_unused_source |= has_unused_source(&native);

            let records = parses(&native);
            assert_eq!(records.len(), 2);
            let expected = if matches!(input, InputCase::Xlsx) {
                profile.first_xlsx_fields
            } else {
                profile.first_text_fields
            };
            let assigned = records[0]
                .assignment
                .fields
                .iter()
                .map(|field| field.name.as_str())
                .collect::<Vec<_>>();
            assert_eq!(assigned, expected);
            assert_first_record_semantics(profile, input, records[0]);
            assert_eq!(
                records[0].review.as_ref().unwrap().status,
                parser_core::RecordReviewStatus::NeedsReview
            );

            assert!(!records[1].assignment.warnings.is_empty());
            assert!(
                records[1]
                    .assignment
                    .warnings
                    .iter()
                    .any(|warning| warning.code == "required_field_missing")
            );
            assert!(records[1].assignment.warnings.iter().any(|warning| {
                matches!(
                    warning.code.as_str(),
                    "multiple_candidates_ambiguous" | "text_field_ambiguous"
                )
            }));
            assert!(!records[1].assignment.unassigned_candidates.is_empty());
            assert_eq!(
                records[1].review.as_ref().unwrap().status,
                parser_core::RecordReviewStatus::NeedsReview
            );
        }
        assert!(
            saw_unused_source,
            "each profile must retain nonempty unused content in the shared corpus"
        );
    }
}

#[test]
fn conformance_profiles_remain_fixture_only_and_dependency_free() {
    let root = repository_root();
    let mut implementation = String::new();
    for crate_name in [
        "parser-core",
        "parser-formats",
        "parser-schema",
        "parser-cli",
    ] {
        let source = root.join("crates").join(crate_name).join("src");
        for entry in fs::read_dir(source).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().and_then(|value| value.to_str()) == Some("rs") {
                implementation.push_str(&fs::read_to_string(path).unwrap());
            }
        }
    }
    for record_name in PROFILE_RECORD_NAMES {
        assert!(!implementation.contains(record_name));
    }

    let dependency_contract = [
        root.join("Cargo.toml"),
        root.join("Cargo.lock"),
        root.join("crates/parser-core/Cargo.toml"),
        root.join("crates/parser-formats/Cargo.toml"),
        root.join("crates/parser-schema/Cargo.toml"),
        root.join("crates/parser-cli/Cargo.toml"),
    ]
    .into_iter()
    .map(|path| fs::read_to_string(path).unwrap())
    .collect::<String>()
    .to_ascii_lowercase();
    for forbidden in ["qualevents", "digital-invitation", "wedding-app"] {
        assert!(!dependency_contract.contains(forbidden));
    }

    for profile in PROFILES {
        let value: Value =
            serde_json::from_str(&fs::read_to_string(fixture(profile.fixture)).unwrap()).unwrap();
        assert!(PROFILE_RECORD_NAMES.contains(&value["record_name"].as_str().unwrap()));
        assert_eq!(value["schema_version"], "0.1");
    }
}

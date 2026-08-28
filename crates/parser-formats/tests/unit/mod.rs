use super::*;
use parser_core::RawValue;
use std::{io::Cursor, path::PathBuf};

#[test]
fn extracts_lines_without_normalizing_content() {
    let document = read_txt_bytes(Some("sample.txt"), b"Ada  \n\n Grace\r\n", "sample.txt")
        .expect("valid text should be read");

    assert_eq!(document.blocks.len(), 3);
    assert_eq!(document.blocks[0].value, RawValue::text("Ada  "));
    assert_eq!(document.blocks[1].value, RawValue::text(""));
    assert_eq!(document.blocks[2].value, RawValue::text(" Grace"));
    assert_eq!(document.blocks[2].location.line, Some(3));
    assert_eq!(document.blocks[2].location.byte_start, Some(7));
    assert_eq!(document.blocks[2].location.byte_end, Some(13));
}

#[test]
fn text_and_stdin_use_the_same_block_extraction() {
    let content = "Ada Lovelace\nGrace Hopper";
    let text_document =
        read_input(InputSource::Text(content), TextLimits::default()).expect("text should be read");
    let mut stdin = Cursor::new(content.as_bytes());
    let stdin_document = read_input(InputSource::Stdin(&mut stdin), TextLimits::default())
        .expect("stdin should be read");

    assert_eq!(text_document.blocks, stdin_document.blocks);
    assert_eq!(text_document.source.source_type, SourceType::Text);
    assert_eq!(stdin_document.source.source_type, SourceType::Stdin);
}

#[test]
fn detects_comma_delimiter_and_cell_provenance() {
    let document = read_csv_bytes(
        Some("sample.csv"),
        b"name,email\nAda,ada@example.test\n",
        "sample.csv",
        CsvOptions::default(),
    )
    .expect("comma CSV should be read");

    assert_eq!(document.source.delimiter.as_deref(), Some(","));
    assert_eq!(document.blocks.len(), 4);
    assert_eq!(document.blocks[2].value, RawValue::text("Ada"));
    assert_eq!(document.blocks[2].location.row, Some(2));
    assert_eq!(document.blocks[2].location.column, Some(1));
}

#[test]
fn supports_semicolon_and_multiline_quoted_cells() {
    let document = read_csv_bytes(
        None,
        b"name;note\nAda;\"line one\nline two\"\nGrace;;\n",
        "messy.csv",
        CsvOptions::default(),
    )
    .expect("semicolon CSV should be read");

    assert_eq!(document.source.delimiter.as_deref(), Some(";"));
    assert_eq!(
        document.blocks[3].value,
        RawValue::text("line one\nline two")
    );
    assert_eq!(document.blocks[3].location.row, Some(2));
    assert_eq!(document.blocks[5].value, RawValue::text(""));
}

#[test]
fn supports_explicit_delimiter_override() {
    let document = read_csv_bytes(
        None,
        b"left;right\n1;2\n",
        "values.csv",
        CsvOptions::with_delimiter(CsvDelimiter::Comma),
    )
    .expect("explicit delimiter should be honored");

    assert_eq!(document.source.delimiter.as_deref(), Some(","));
    assert_eq!(document.blocks.len(), 2);
    assert_eq!(document.blocks[0].value, RawValue::text("left;right"));
}

#[test]
fn detects_pipe_delimiter() {
    let document = read_csv_bytes(None, b"a|b\n1|2\n", "values.psv", CsvOptions::default())
        .expect("pipe-delimited data should be read");

    assert_eq!(document.source.delimiter.as_deref(), Some("|"));
}

#[test]
fn rejects_malformed_csv_structurally() {
    let error = read_csv_bytes(
        None,
        b"name,note\nAda,\"unclosed\n",
        "broken.csv",
        CsvOptions::default(),
    )
    .expect_err("malformed CSV should fail");

    assert_eq!(error.code(), "invalid_csv");
}

#[test]
fn reads_xlsx_fixture_with_typed_cells_and_sheet_provenance() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/xlsx/sample.xlsx");
    let document = read_xlsx(path).expect("XLSX fixture should be readable");

    assert_eq!(document.source.source_type, SourceType::Xlsx);
    assert_eq!(document.source.file_name.as_deref(), Some("sample.xlsx"));
    assert_eq!(document.blocks.len(), 12);
    assert_eq!(document.blocks[4].value, RawValue::Text("Ada".to_owned()));
    assert_eq!(document.blocks[5].value, RawValue::Decimal(42.0));
    assert_eq!(document.blocks[6].value, RawValue::Boolean(true));
    assert_eq!(document.blocks[7].value, RawValue::DateTime(45943.5));
    assert_eq!(document.blocks[10].value, RawValue::Null);
    assert_eq!(document.blocks[5].location.sheet.as_deref(), Some("Data"));
    assert_eq!(document.blocks[5].location.row, Some(2));
    assert_eq!(document.blocks[5].location.column, Some(2));
}

fn expected_xlsx_sample() -> RawDocument {
    let values = [
        RawValue::text("Name"),
        RawValue::text("Count"),
        RawValue::text("Enabled"),
        RawValue::text("Date"),
        RawValue::text("Ada"),
        RawValue::Decimal(42.0),
        RawValue::Boolean(true),
        RawValue::DateTime(45943.5),
        RawValue::text("Title"),
        RawValue::text("Merged"),
        RawValue::Null,
        RawValue::Null,
    ];
    RawDocument {
        id: "xlsx-document".to_owned(),
        source: SourceMetadata {
            source_type: SourceType::Xlsx,
            file_name: Some("sample.xlsx".to_owned()),
            mime_type: Some(
                "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet".to_owned(),
            ),
            size_bytes: Some(2027),
            delimiter: None,
        },
        blocks: values
            .into_iter()
            .enumerate()
            .map(|(index, value)| RawBlock {
                id: format!("sheet-1-row-{}-column-{}", index / 4 + 1, index % 4 + 1),
                value,
                location: SourceLocation {
                    row: Some(index / 4 + 1),
                    column: Some(index % 4 + 1),
                    sheet: Some("Data".to_owned()),
                    ..SourceLocation::default()
                },
            })
            .collect(),
        warnings: Vec::new(),
    }
}

#[test]
fn xlsx_file_reader_preserves_complete_fixture_contract() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/xlsx/sample.xlsx");
    assert_eq!(read_xlsx(path).unwrap(), expected_xlsx_sample());
}

#[test]
fn xlsx_file_errors_preserve_supplied_paths_and_categories() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures");
    let missing = root.join("xlsx/does-not-exist.xlsx");
    assert_eq!(
        read_xlsx(&missing).unwrap_err(),
        ParserError::Io {
            path: missing.to_string_lossy().into_owned(),
            kind: parser_core::IoErrorKind::NotFound,
        }
    );
    let invalid = root.join("text/simple.txt");
    assert_eq!(
        read_xlsx(&invalid).unwrap_err(),
        ParserError::InvalidXlsx {
            path: invalid.to_string_lossy().into_owned(),
            message: "Zip(InvalidArchive(\"Could not find EOCD\"))".to_owned(),
        }
    );
}

// A hex-encoded synthetic XLSX fixture stores the archive as plain text without a
// new ZIP/base64 test dependency. Runtime byte tests never open a workbook path.
fn unicode_xlsx_bytes() -> Vec<u8> {
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/xlsx/unicode.xlsx.hex"
    ))
    .split_ascii_whitespace()
    .map(|byte| u8::from_str_radix(byte, 16).expect("fixture contains hex bytes"))
    .collect()
}

#[test]
fn xlsx_bytes_match_file_contract_and_repeat_deterministically() {
    let bytes = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/xlsx/sample.xlsx"
    ));
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/xlsx/sample.xlsx");
    let from_file = read_xlsx(path).unwrap();
    for _ in 0..3 {
        let from_bytes = read_xlsx_bytes(Some("sample.xlsx"), bytes).unwrap();
        assert_eq!(from_bytes, expected_xlsx_sample());
        assert_eq!(from_bytes, from_file);
    }
}

#[test]
fn xlsx_bytes_preserve_unicode_and_optional_filename_metadata() {
    let bytes = unicode_xlsx_bytes();
    assert_eq!(bytes.len(), 2121);
    let mut expected = expected_xlsx_sample();
    expected.source.size_bytes = Some(2121);
    expected.blocks[4].value = RawValue::text("  Zoë 東京 😀  ");
    expected.blocks[8].value = RawValue::text("  untouched  ");
    expected.blocks[9].value = RawValue::text("  ada@example.test grace@example.test  ");
    for block in &mut expected.blocks {
        block.location.sheet = Some("記録 😀".to_owned());
    }
    // The numeric cell has formula 1+1 with cached value 42: do not evaluate it.
    for file_name in [
        None,
        Some("résumé 東京 😀.xlsx"),
        Some("absent/directory/input.xlsx"),
    ] {
        expected.source.file_name = file_name.map(str::to_owned);
        let document = read_xlsx_bytes(file_name, &bytes).unwrap();
        assert_eq!(document, expected);
        assert_eq!(read_xlsx_bytes(file_name, &bytes).unwrap(), document);
    }
}

#[test]
fn xlsx_bytes_reject_invalid_archives_without_input_diagnostics() {
    let bytes = unicode_xlsx_bytes();
    for invalid in [
        &[][..],
        &bytes[..2],
        &bytes[..bytes.len() / 2],
        &bytes[..bytes.len() - 22],
        b"synthetic-private-workbook-content".as_slice(),
    ] {
        for file_name in [None, Some("synthetic-private-filename.xlsx")] {
            let error = read_xlsx_bytes(file_name, invalid).unwrap_err();
            assert_eq!(
                error,
                ParserError::InvalidXlsx {
                    path: String::new(),
                    message: "could not read XLSX workbook".to_owned(),
                }
            );
            assert_eq!(error.code(), "invalid_xlsx");
            assert_eq!(read_xlsx_bytes(file_name, invalid).unwrap_err(), error);
        }
    }
}

#[test]
fn xlsx_byte_document_retains_parse_source_evidence_and_review() {
    use parser_core::{
        AssignmentField, CandidateType, ParseContent, RecordReviewStatus, SourceBlockRole,
        TextSpan, parse_document_with_assignment,
    };

    let document = read_xlsx_bytes(None, &unicode_xlsx_bytes()).unwrap();
    let fields = [
        ("Count", CandidateType::Integer),
        ("Enabled", CandidateType::Boolean),
    ]
    .map(|(name, candidate_type)| AssignmentField {
        name: name.to_owned(),
        aliases: Vec::new(),
        candidate_type,
        required: true,
        multiple: false,
        unique: false,
        constraints: Vec::new(),
        expected_column: None,
    });
    let result = parse_document_with_assignment(&document, &fields, &[], None);
    assert_eq!(
        result,
        parse_document_with_assignment(&document, &fields, &[], None)
    );
    assert_eq!(result.warnings, document.warnings);
    let evidence = result.source_evidence.as_ref().unwrap();
    assert_eq!(evidence.document, document);
    let ParseContent::Table { sheets } = &result.content else {
        panic!("XLSX coordinates must select the table path");
    };
    assert_eq!(sheets.len(), 1);
    assert_eq!(sheets[0].sheet.as_deref(), Some("記録 😀"));
    let records = &sheets[0].records;
    assert_eq!(records.len(), 2);
    assert_eq!(
        records[0].parse.assignment.fields[0].candidates[0].raw_value,
        "42"
    );
    assert_eq!(
        records[0].parse.assignment.fields[1].candidates[0].raw_value,
        "true"
    );
    assert!(!records[0].parse.assignment.unassigned_candidates.is_empty());
    assert!(!records[1].parse.assignment.unassigned_candidates.is_empty());
    assert!(
        records[1]
            .parse
            .assignment
            .warnings
            .iter()
            .any(|warning| warning.code == "required_field_missing")
    );
    assert_eq!(
        records[1].parse.review.as_ref().unwrap().status,
        RecordReviewStatus::NeedsReview
    );

    let mut covered: Vec<Vec<bool>> = document
        .blocks
        .iter()
        .map(|block| vec![false; block.value.to_text().len()])
        .collect();
    for record in records {
        let parse = &record.parse;
        for candidate in parse
            .candidates
            .iter()
            .chain(
                parse
                    .assignment
                    .fields
                    .iter()
                    .flat_map(|field| &field.candidates),
            )
            .chain(&parse.assignment.unassigned_candidates)
        {
            let reference = candidate.source_reference.as_ref().unwrap();
            assert_eq!(
                reference.resolve(&evidence.document).as_deref(),
                Some(candidate.raw_value.as_str())
            );
            assert!(
                parse
                    .candidates
                    .iter()
                    .any(
                        |detected| detected.source_reference == candidate.source_reference
                            && detected.candidate_type == candidate.candidate_type
                    )
            );
            covered[reference.block_index][reference.span.byte_start..reference.span.byte_end]
                .fill(true);
        }
    }
    assert_eq!(evidence.blocks.len(), document.blocks.len());
    for (index, coverage) in evidence.blocks.iter().enumerate() {
        assert_eq!(coverage.block_index, index);
        if index < 4 {
            assert_eq!(coverage.role, SourceBlockRole::Header);
            assert!(coverage.reason.is_some());
            continue;
        }
        assert_eq!(coverage.role, SourceBlockRole::Parsed);
        for span in &coverage.unused_spans {
            assert!(
                document.blocks[index]
                    .value
                    .to_text()
                    .get(span.byte_start..span.byte_end)
                    .is_some()
            );
            let bytes = &mut covered[index][span.byte_start..span.byte_end];
            assert!(bytes.iter().all(|byte| !byte));
            bytes.fill(true);
        }
        assert!(
            covered[index].iter().all(|byte| *byte),
            "all source bytes must be accounted for"
        );
    }
    assert_eq!(
        evidence.blocks[4].unused_spans,
        vec![TextSpan {
            byte_start: 0,
            byte_end: "  Zoë 東京 😀  ".len()
        }]
    );
    assert_eq!(
        evidence.blocks[10].unused_spans,
        vec![TextSpan {
            byte_start: 0,
            byte_end: 0
        }]
    );
}

#[test]
fn rejects_invalid_xlsx_structurally() {
    let error = read_xlsx("fixtures/xlsx/does-not-exist.xlsx")
        .expect_err("missing XLSX should return an error");

    assert_eq!(error.code(), "io_error");
}

#[test]
fn rejects_input_that_exceeds_byte_limit() {
    let error = read_input(InputSource::Text("12345"), TextLimits::new(4, 100))
        .expect_err("oversized text should be rejected");

    assert_eq!(
        error,
        ParserError::InputTooLarge {
            source: "<text>".to_owned(),
            limit: 4,
            actual: 5,
        }
    );
}

#[test]
fn enforces_byte_limit_while_reading_stdin() {
    let mut stdin = Cursor::new(b"12345".to_vec());
    let error = read_input(InputSource::Stdin(&mut stdin), TextLimits::new(4, 100))
        .expect_err("oversized stdin should be rejected");

    assert_eq!(
        error,
        ParserError::InputTooLarge {
            source: "<stdin>".to_owned(),
            limit: 4,
            actual: 5,
        }
    );
}

#[test]
fn rejects_line_that_exceeds_line_limit() {
    let error = read_input(InputSource::Text("12345"), TextLimits::new(100, 4))
        .expect_err("long line should be rejected");

    assert_eq!(
        error,
        ParserError::LineTooLong {
            source: "<text>".to_owned(),
            line: 1,
            limit: 4,
            actual: 5,
        }
    );
}

#[test]
fn rejects_invalid_utf8_with_byte_offset() {
    let error = read_txt_bytes(None, b"ok\xFF", "broken.txt").expect_err("invalid UTF-8");

    assert_eq!(
        error,
        ParserError::InvalidUtf8 {
            path: "broken.txt".to_owned(),
            valid_up_to: 2,
        }
    );
}

#[test]
fn reads_repository_fixture() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/text/simple.txt");
    let document = read_txt(path).expect("fixture should be valid UTF-8");

    assert_eq!(document.source.file_name.as_deref(), Some("simple.txt"));
    assert_eq!(document.blocks.len(), 2);
    assert_eq!(document.blocks[0].value, RawValue::text("Ada Lovelace"));
    assert_eq!(document.blocks[1].value, RawValue::text("Grace Hopper"));
}

#[test]
fn missing_file_is_structured_as_an_io_error() {
    let error = read_txt("fixtures/text/does-not-exist.txt")
        .expect_err("missing file should return an error");

    assert_eq!(error.code(), "io_error");
}

#[test]
fn csv_document_parses_with_header_driven_assignment() {
    let bytes = b"Email,Age\nada@example.test,30\ngrace@example.test,45\n";
    let document = read_csv_bytes(
        Some("people.csv"),
        bytes,
        "people.csv",
        CsvOptions::default(),
    )
    .expect("csv should parse");
    let fields = [parser_core::AssignmentField {
        name: "email".to_owned(),
        aliases: vec!["contact".to_owned()],
        candidate_type: parser_core::CandidateType::Email,
        required: true,
        multiple: false,
        unique: false,
        constraints: Vec::new(),
        expected_column: None,
    }];

    let result = parser_core::parse_document_rows_with_assignment(&document, &fields, &[]);

    assert_eq!(result.warnings.len(), 0);
    assert_eq!(result.sheets.len(), 1);
    let sheet = &result.sheets[0];
    assert!(sheet.header.context().is_some());
    assert_eq!(
        sheet.header.context().unwrap().labels,
        vec![(1, "Email".to_owned()), (2, "Age".to_owned())]
    );
    assert_eq!(sheet.records.len(), 2);
    for record in &sheet.records {
        let email = &record.parse.assignment.fields[0].candidates[0];
        assert_eq!(email.candidate_type, parser_core::CandidateType::Email);
        assert_eq!(email.source_column, Some(1));
        assert!(
            email
                .reasons
                .iter()
                .any(|reason| reason.code == "header_label_match")
        );
    }
    assert_eq!(
        sheet.records[1].parse.assignment.fields[0].candidates[0].raw_value,
        "grace@example.test"
    );
}

#[test]
fn empty_formats_test() {
    assert!(formats_ready());
}

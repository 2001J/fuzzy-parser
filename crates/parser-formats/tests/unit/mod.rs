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

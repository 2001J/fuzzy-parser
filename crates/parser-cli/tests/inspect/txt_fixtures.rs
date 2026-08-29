use super::support;
use serde_json::{Value, json};

fn decode_hex(encoded: &str) -> Vec<u8> {
    encoded
        .split_ascii_whitespace()
        .map(|byte| u8::from_str_radix(byte, 16).expect("fixture contains hex bytes"))
        .collect()
}

fn inspect_hex(name: &str, encoded: &str) -> (std::process::Output, Value) {
    let directory = support::TestDirectory::new();
    let path = directory.file(name, &decode_hex(encoded));
    let output = support::run(&["inspect", path.to_str().unwrap()], None);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stderr.is_empty());
    let document: Value = serde_json::from_slice(&output.stdout).expect("stdout is one JSON value");
    (output, document)
}

fn expected_document(name: &str, size: u64, lines: &[(&str, usize, usize)]) -> Value {
    json!({
        "id": "txt-document",
        "source": {
            "source_type": "txt",
            "file_name": name,
            "mime_type": "text/plain",
            "size_bytes": size,
            "delimiter": null
        },
        "blocks": lines.iter().enumerate().map(|(index, (value, start, end))| json!({
            "id": format!("block-{}", index + 1),
            "value": {"kind": "Text", "value": value},
            "location": {
                "line": index + 1,
                "row": null,
                "column": null,
                "sheet": null,
                "byte_start": start,
                "byte_end": end
            }
        })).collect::<Vec<_>>(),
        "warnings": []
    })
}

fn assert_exact_output(output: &std::process::Output, document: &Value, expected: Value) {
    assert_eq!(document, &expected);
    assert_eq!(output.stdout.first(), Some(&b'{'));
    assert_eq!(output.stdout.last(), Some(&b'\n'));
    // `from_slice` above rejects any non-whitespace progress/decorative tail.
}

#[test]
fn cli_preserves_unicode_punctuation_and_raw_whitespace_fixture() {
    let (output, document) = inspect_hex(
        "unicode-whitespace.txt",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/text/unicode-whitespace.txt.hex"
        )),
    );
    assert_exact_output(
        &output,
        &document,
        expected_document(
            "unicode-whitespace.txt",
            65,
            &[
                ("  Zoë—東京 😀\t ", 0, 22),
                ("\t“Élan”; +1 (202) 555-0100\u{a0}  ", 23, 58),
                ("Cafe\u{301}", 59, 65),
            ],
        ),
    );
}

#[test]
fn cli_preserves_lf_crlf_consecutive_blanks_and_trailing_newlines() {
    let cases = [
        (
            "blank-lines-lf.txt",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../fixtures/text/blank-lines-lf.txt.hex"
            )),
            17,
            vec![
                ("", 0, 0),
                ("", 1, 1),
                ("Alpha", 2, 7),
                ("", 8, 8),
                ("", 9, 9),
                ("Omega", 10, 15),
                ("", 16, 16),
            ],
        ),
        (
            "blank-lines-crlf.txt",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../fixtures/text/blank-lines-crlf.txt.hex"
            )),
            24,
            vec![
                ("", 0, 0),
                ("", 2, 2),
                ("Alpha", 4, 9),
                ("", 11, 11),
                ("", 13, 13),
                ("Omega", 15, 20),
                ("", 22, 22),
            ],
        ),
    ];
    for (name, encoded, size, lines) in cases {
        let (output, document) = inspect_hex(name, encoded);
        assert_exact_output(&output, &document, expected_document(name, size, &lines));
    }
}

#[test]
fn cli_materializes_invalid_utf8_fixture_and_reports_its_exact_offset() {
    let directory = support::TestDirectory::new();
    let path = directory.file(
        "invalid-utf8.txt",
        &decode_hex(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/text/invalid-utf8.txt.hex"
        ))),
    );
    let output = support::run(&["inspect", path.to_str().unwrap()], None);
    assert_eq!(
        support::error(&output),
        json!({
            "error": {
                "error_contract_version": "0.1",
                "code": "invalid_utf8",
                "valid_up_to": 5
            },
            "message": "input is not valid UTF-8 at byte offset 5"
        })
    );
}

use crate::{InputSource, TextLimits, read_input, read_limited, read_txt};
use parser_core::{
    IoErrorKind, ParserError, RawBlock, RawDocument, RawValue, SourceLocation, SourceMetadata,
    SourceType,
};
use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};

struct TempDirectory(PathBuf);

impl TempDirectory {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        for _ in 0..100 {
            let path = std::env::temp_dir().join(format!(
                "fuzzy-parser-txt-fixtures-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("could not create fixture directory: {error}"),
            }
        }
        panic!("could not allocate a unique fixture directory");
    }

    fn write_hex_fixture(&self, name: &str) -> PathBuf {
        let encoded = fs::read_to_string(fixture_path(&format!("{name}.hex"))).unwrap();
        let bytes: Vec<u8> = encoded
            .split_ascii_whitespace()
            .map(|byte| u8::from_str_radix(byte, 16).expect("fixture contains hex bytes"))
            .collect();
        let path = self.0.join(name);
        fs::write(&path, bytes).unwrap();
        path
    }
}

impl Drop for TempDirectory {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).expect("remove owned fixture directory");
    }
}

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/text")
        .join(name)
}

fn assert_document(path: &Path, size_bytes: u64, lines: &[(&str, usize, usize)]) {
    let expected = RawDocument {
        id: "txt-document".to_owned(),
        source: SourceMetadata {
            source_type: SourceType::Txt,
            file_name: Some(path.file_name().unwrap().to_str().unwrap().to_owned()),
            mime_type: Some("text/plain".to_owned()),
            size_bytes: Some(size_bytes),
            delimiter: None,
        },
        blocks: lines
            .iter()
            .enumerate()
            .map(|(index, &(text, start, end))| RawBlock {
                id: format!("block-{}", index + 1),
                value: RawValue::text(text),
                location: SourceLocation {
                    line: Some(index + 1),
                    byte_start: Some(start),
                    byte_end: Some(end),
                    ..SourceLocation::default()
                },
            })
            .collect(),
        warnings: Vec::new(),
    };
    let bytes = fs::read(path).unwrap();
    assert_eq!(bytes.len() as u64, size_bytes);
    for _ in 0..2 {
        let document = read_txt(path).expect("fixture is valid UTF-8");
        assert_eq!(document, expected);
        assert_eq!(
            read_input(InputSource::TxtFile(path), TextLimits::default()).unwrap(),
            document
        );
        for block in &document.blocks {
            let start = block.location.byte_start.unwrap();
            let end = block.location.byte_end.unwrap();
            assert_eq!(&bytes[start..end], block.value.to_text().as_bytes());
        }
    }
}

#[test]
fn unicode_file_preserves_punctuation_whitespace_and_byte_coordinates() {
    let directory = TempDirectory::new();
    let path = directory.write_hex_fixture("unicode-whitespace.txt");
    assert_document(
        &path,
        65,
        &[
            ("  Zoë—東京 😀\t ", 0, 22),
            ("\t“Élan”; +1 (202) 555-0100\u{a0}  ", 23, 58),
            ("Cafe\u{301}", 59, 65),
        ],
    );
}

#[test]
fn empty_file_has_metadata_and_no_blocks() {
    assert_document(&fixture_path("empty.txt"), 0, &[]);
}

#[test]
fn lf_and_crlf_files_preserve_consecutive_blanks_without_a_phantom_final_line() {
    let directory = TempDirectory::new();
    let lf = directory.write_hex_fixture("blank-lines-lf.txt");
    assert_document(
        &lf,
        17,
        &[
            ("", 0, 0),
            ("", 1, 1),
            ("Alpha", 2, 7),
            ("", 8, 8),
            ("", 9, 9),
            ("Omega", 10, 15),
            ("", 16, 16),
        ],
    );
    let crlf = directory.write_hex_fixture("blank-lines-crlf.txt");
    assert_document(
        &crlf,
        24,
        &[
            ("", 0, 0),
            ("", 2, 2),
            ("Alpha", 4, 9),
            ("", 11, 11),
            ("", 13, 13),
            ("Omega", 15, 20),
            ("", 22, 22),
        ],
    );
}

#[test]
fn invalid_utf8_file_reports_the_original_byte_offset() {
    let directory = TempDirectory::new();
    let path = directory.write_hex_fixture("invalid-utf8.txt");
    for _ in 0..2 {
        assert!(matches!(
            read_txt(&path).unwrap_err(),
            ParserError::InvalidUtf8 { valid_up_to: 5, .. }
        ));
    }
}

#[test]
fn missing_file_retains_the_not_found_cause() {
    let directory = TempDirectory::new();
    assert!(matches!(
        read_txt(directory.0.join("missing.txt")).unwrap_err(),
        ParserError::Io {
            kind: IoErrorKind::NotFound,
            ..
        }
    ));
}

#[test]
fn directory_path_retains_the_platform_io_cause() {
    let directory = TempDirectory::new();
    // Opening a directory may succeed on Unix, but reading it as a file fails.
    // Compare the typed OS cause instead of pinning a platform-specific errno.
    let expected_kind = fs::read(&directory.0).unwrap_err().kind().into();
    let ParserError::Io { kind, .. } = read_txt(&directory.0).unwrap_err() else {
        panic!("unreadable path must return a structured I/O error");
    };
    assert_eq!(kind, expected_kind);
}

#[test]
fn permission_denied_after_partial_read_retains_the_cause_not_partial_content() {
    struct PermissionDenied;
    impl Read for PermissionDenied {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::ErrorKind::PermissionDenied.into())
        }
    }

    // Exercise the same bounded reader used by TXT files without chmod/root
    // assumptions. This tests error propagation, not filesystem permissions.
    let mut reader = io::Cursor::new(b"partial\n").chain(PermissionDenied);
    assert!(matches!(
        read_limited(&mut reader, "synthetic.txt", TextLimits::default()).unwrap_err(),
        ParserError::Io {
            kind: IoErrorKind::PermissionDenied,
            ..
        }
    ));
}

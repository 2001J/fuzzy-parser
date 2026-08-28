use super::enabled_format;
use crate::tests::txt_fixtures::TempDirectory;
use crate::*;

#[test]
fn txt_paths_reject_unrecognized_extensions_before_decoding() {
    let directory = TempDirectory::new();
    for name in [
        "input.pdf",
        "input",
        "input.csv",
        "input.xlsx",
        "input.txt.exe",
    ] {
        let path = directory.0.join(name);
        fs::write(&path, b"hello").unwrap();
        assert_eq!(read_txt(&path).unwrap_err().code(), "unsupported_input");
        assert_eq!(
            read_input(InputSource::TxtFile(&path), TextLimits::default())
                .unwrap_err()
                .code(),
            "unsupported_input"
        );
    }
}

#[test]
fn directory_is_rejected_as_nonregular_before_open() {
    let directory = TempDirectory::new();
    assert_eq!(
        read_txt(&directory.0).unwrap_err().code(),
        "not_regular_file"
    );
}

#[test]
fn metadata_oversize_precedes_utf8_and_line_decoding() {
    let directory = TempDirectory::new();
    let path = directory.0.join("large.txt");
    fs::write(&path, b"\xff\xff\xff\xff\xff").unwrap();
    assert_eq!(
        read_input(InputSource::TxtFile(&path), TextLimits::new(4, 1))
            .unwrap_err()
            .code(),
        "file_too_large"
    );
}

#[test]
fn enabled_formats_return_an_unread_handle_without_content_sniffing() {
    let directory = TempDirectory::new();
    let options = FileValidationOptions::default();
    assert_eq!(options.max_bytes, 1024 * 1024);
    assert_eq!(options.empty_policy, EmptyFilePolicy::Accept);
    for (name, format) in [
        ("input.TXT", FileFormat::Txt),
        ("input.CsV", FileFormat::Csv),
        ("input.XLSX", FileFormat::Xlsx),
    ] {
        let path = directory.0.join(name);
        fs::write(&path, b"not a workbook\xff").unwrap();
        let validated = open_validated_file(&path, &options).unwrap();
        assert_eq!(validated.format, format);
        assert_eq!(validated.size_bytes, 15);
        let mut bytes = Vec::new();
        validated.into_file().read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, b"not a workbook\xff");
        let disabled = FileValidationOptions {
            enabled_formats: vec![],
            ..options.clone()
        };
        assert_eq!(
            open_validated_file(&path, &disabled).unwrap_err().code(),
            "unsupported_input"
        );
    }
    let csv = directory.0.join("input.CsV");
    let txt_only = FileValidationOptions {
        enabled_formats: vec![FileFormat::Txt],
        ..options
    };
    assert_eq!(
        open_validated_file(csv, &txt_only).unwrap_err().code(),
        "unsupported_input"
    );
}

#[test]
fn uppercase_txt_and_existing_successful_document_are_unchanged() {
    let directory = TempDirectory::new();
    let path = directory.0.join("source.TXT");
    let bytes = "  Zoë\r\n\n東京  ".as_bytes();
    fs::write(&path, bytes).unwrap();
    let expected = read_txt_bytes(Some("source.TXT"), bytes, "ignored").unwrap();
    assert_eq!(read_txt(&path).unwrap(), expected);
    assert_eq!(
        read_txt_with_options(&path, TxtOptions::default()).unwrap(),
        expected
    );
    assert_eq!(
        read_input(InputSource::TxtFile(&path), TextLimits::default()).unwrap(),
        expected
    );
}

#[test]
fn empty_policy_is_explicit_and_whitespace_is_not_empty() {
    let directory = TempDirectory::new();
    let path = directory.0.join("empty.txt");
    fs::write(&path, b"").unwrap();
    let defaults = TxtOptions::default();
    assert_eq!(defaults.empty_policy, EmptyFilePolicy::Accept);
    assert!(read_txt(&path).unwrap().blocks.is_empty());
    let reject = TxtOptions {
        empty_policy: EmptyFilePolicy::Reject,
        ..defaults
    };
    assert_eq!(
        read_txt_with_options(&path, reject).unwrap_err(),
        ParserError::EmptyInput {
            path: path.to_string_lossy().into_owned()
        }
    );
    for bytes in [b" ".as_slice(), b"\t\r\n"] {
        fs::write(&path, bytes).unwrap();
        assert_eq!(
            read_txt_with_options(&path, reject)
                .unwrap()
                .source
                .size_bytes,
            Some(bytes.len() as u64)
        );
    }
}

#[test]
fn zero_exact_and_one_over_size_limits_are_enforced() {
    let directory = TempDirectory::new();
    let path = directory.0.join("size.txt");
    let options = TxtOptions {
        limits: TextLimits::new(0, 0),
        ..TxtOptions::default()
    };
    fs::write(&path, b"").unwrap();
    assert!(
        read_txt_with_options(&path, options)
            .unwrap()
            .blocks
            .is_empty()
    );
    assert_eq!(
        read_txt_with_options(
            &path,
            TxtOptions {
                empty_policy: EmptyFilePolicy::Reject,
                ..options
            }
        )
        .unwrap_err()
        .code(),
        "empty_input"
    );
    fs::write(&path, b"a").unwrap();
    assert_eq!(
        read_txt_with_options(&path, options).unwrap_err(),
        ParserError::FileTooLarge {
            path: path.to_string_lossy().into_owned(),
            limit: 0,
            actual: 1
        }
    );
    let options = TxtOptions {
        limits: TextLimits::new(4, 4),
        ..TxtOptions::default()
    };
    fs::write(&path, b"1234").unwrap();
    assert_eq!(
        read_txt_with_options(&path, options).unwrap().blocks[0].value,
        RawValue::text("1234")
    );
    fs::write(&path, b"12345").unwrap();
    assert_eq!(
        read_txt_with_options(&path, options).unwrap_err(),
        ParserError::FileTooLarge {
            path: path.to_string_lossy().into_owned(),
            limit: 4,
            actual: 5
        }
    );
}

#[test]
fn metadata_rejects_before_full_read_and_retains_exact_size() {
    let directory = TempDirectory::new();
    let path = directory.0.join("sparse.txt");
    let file = std::fs::File::create(&path).unwrap();
    // A small sparse file needs no full contents just to validate its size.
    file.set_len(1024 * 1024 + 23).unwrap();
    assert_eq!(
        open_validated_file(&path, &FileValidationOptions::default()).unwrap_err(),
        ParserError::FileTooLarge {
            path: path.to_string_lossy().into_owned(),
            limit: 1048576,
            actual: 1048599,
        }
    );
}

#[test]
fn growth_after_open_uses_bounded_read_error_not_stale_metadata() {
    let directory = TempDirectory::new();
    let path = directory.0.join("growth.txt");
    fs::write(&path, b"old").unwrap();
    let limits = TextLimits::new(4, 100);
    let input = open_validated_file(
        &path,
        &FileValidationOptions {
            max_bytes: 4,
            ..FileValidationOptions::default()
        },
    )
    .unwrap();
    assert_eq!(input.size_bytes, 3);
    fs::write(&path, b"123456789").unwrap();
    assert_eq!(
        read_validated_txt(
            input,
            &path,
            TxtOptions {
                limits,
                ..TxtOptions::default()
            }
        )
        .unwrap_err(),
        ParserError::InputTooLarge {
            source: path.to_string_lossy().into_owned(),
            limit: 4,
            actual: 5,
        }
    );
}

#[test]
fn shrink_after_open_checks_actual_empty_policy_and_metadata() {
    let directory = TempDirectory::new();
    let path = directory.0.join("shrink.txt");
    for empty_policy in [EmptyFilePolicy::Accept, EmptyFilePolicy::Reject] {
        fs::write(&path, b"old").unwrap();
        let input = open_validated_file(
            &path,
            &FileValidationOptions {
                empty_policy,
                ..FileValidationOptions::default()
            },
        )
        .unwrap();
        assert_eq!(input.size_bytes, 3);
        fs::write(&path, b"").unwrap();
        let result = read_validated_txt(
            input,
            &path,
            TxtOptions {
                empty_policy,
                ..TxtOptions::default()
            },
        );
        if empty_policy == EmptyFilePolicy::Reject {
            assert_eq!(
                result.unwrap_err(),
                ParserError::EmptyInput {
                    path: path.to_string_lossy().into_owned()
                }
            );
        } else {
            assert_eq!(
                result.unwrap(),
                read_txt_bytes(Some("shrink.txt"), b"", "ignored").unwrap()
            );
        }
    }
}

#[test]
fn bounded_reader_consumes_only_limit_plus_one_bytes() {
    struct CountingReader {
        read: usize,
    }
    impl Read for CountingReader {
        fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
            buffer.fill(b'x');
            self.read += buffer.len();
            Ok(buffer.len())
        }
    }
    for limit in [0, 1, 4, 100] {
        let mut reader = CountingReader { read: 0 };
        let error =
            read_limited(&mut reader, "synthetic.txt", TextLimits::new(limit, 100)).unwrap_err();
        assert_eq!(reader.read, limit + 1);
        assert_eq!(
            error,
            ParserError::InputTooLarge {
                source: "synthetic.txt".into(),
                limit,
                actual: limit + 1
            }
        );
    }
}

#[test]
fn missing_path_and_non_file_inputs_retain_separate_semantics() {
    let directory = TempDirectory::new();
    assert!(matches!(
        open_validated_file(
            directory.0.join("missing.txt"),
            &FileValidationOptions::default()
        )
        .unwrap_err(),
        ParserError::Io {
            kind: parser_core::IoErrorKind::NotFound,
            ..
        }
    ));
    // Filename metadata for byte inputs and text/stdin are not file policy.
    assert!(
        read_txt_bytes(Some("anything.pdf"), b"", "ignored")
            .unwrap()
            .blocks
            .is_empty()
    );
    assert!(
        read_input(InputSource::Text(""), TextLimits::default())
            .unwrap()
            .blocks
            .is_empty()
    );
    assert!(
        read_input(
            InputSource::Stdin(&mut Cursor::new(b"")),
            TextLimits::default()
        )
        .unwrap()
        .blocks
        .is_empty()
    );
}

#[cfg(unix)]
#[test]
fn non_utf8_extension_is_rejected_without_decoding_contents() {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};
    // macOS cannot create this filename; exercise OS-path extension selection
    // directly instead of claiming unsupported filesystem setup passed.
    let path = std::path::PathBuf::from(OsString::from_vec(b"input.\xff".to_vec()));
    assert_eq!(
        enabled_format(&path, &FileValidationOptions::default())
            .unwrap_err()
            .code(),
        "unsupported_input"
    );
}

#[cfg(target_os = "linux")]
#[test]
fn non_utf8_file_extension_is_rejected_by_both_txt_entry_points() {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};
    let directory = TempDirectory::new();
    let path = directory.0.join(OsString::from_vec(b"input.\xff".to_vec()));
    fs::write(&path, b"\xff").unwrap();
    assert_eq!(read_txt(&path).unwrap_err().code(), "unsupported_input");
    assert_eq!(
        read_input(InputSource::TxtFile(&path), TextLimits::default())
            .unwrap_err()
            .code(),
        "unsupported_input"
    );
}

#[cfg(unix)]
#[test]
fn special_socket_is_rejected_without_attempting_a_file_read() {
    let directory = TempDirectory::new();
    let path = directory.0.join("s");
    let _socket = std::os::unix::net::UnixListener::bind(&path).unwrap();
    assert_eq!(
        open_validated_file(&path, &FileValidationOptions::default())
            .unwrap_err()
            .code(),
        "not_regular_file"
    );
}

#[cfg(unix)]
#[test]
fn symlinks_follow_regular_targets_and_preserve_supplied_name() {
    use std::os::unix::fs::symlink;
    let directory = TempDirectory::new();
    let target = directory.0.join("target.bin");
    fs::write(&target, b"original").unwrap();
    let link = directory.0.join("alias.TXT");
    symlink(&target, &link).unwrap();
    assert_eq!(
        read_txt(&link).unwrap(),
        read_txt_bytes(Some("alias.TXT"), b"original", "ignored").unwrap()
    );
    let dir_link = directory.0.join("directory.txt");
    symlink(&directory.0, &dir_link).unwrap();
    assert_eq!(read_txt(&dir_link).unwrap_err().code(), "not_regular_file");
    let dangling = directory.0.join("dangling.txt");
    symlink(directory.0.join("absent"), &dangling).unwrap();
    assert!(matches!(
        read_txt(&dangling).unwrap_err(),
        ParserError::Io {
            kind: parser_core::IoErrorKind::NotFound,
            ..
        }
    ));
}

#[cfg(unix)]
#[test]
fn extraction_does_not_reopen_a_replaced_path() {
    let directory = TempDirectory::new();
    let path = directory.0.join("original.txt");
    fs::write(&path, b"original").unwrap();
    let input = open_validated_file(&path, &FileValidationOptions::default()).unwrap();
    fs::rename(&path, directory.0.join("moved.txt")).unwrap();
    fs::write(&path, b"replacement").unwrap();
    assert_eq!(
        read_validated_txt(input, &path, TxtOptions::default()).unwrap(),
        read_txt_bytes(Some("original.txt"), b"original", "ignored").unwrap()
    );
}

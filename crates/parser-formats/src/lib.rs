use parser_core::{
    ParserError, RawBlock, RawDocument, RawValue, SourceLocation, SourceMetadata, SourceType,
};
use std::{fs::File, io::Read, path::Path};

pub const DEFAULT_MAX_TEXT_BYTES: usize = 1024 * 1024;
pub const DEFAULT_MAX_LINE_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextLimits {
    pub max_bytes: usize,
    pub max_line_bytes: usize,
}

impl TextLimits {
    pub const fn new(max_bytes: usize, max_line_bytes: usize) -> Self {
        Self {
            max_bytes,
            max_line_bytes,
        }
    }
}

impl Default for TextLimits {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_TEXT_BYTES, DEFAULT_MAX_LINE_BYTES)
    }
}

pub enum InputSource<'a> {
    Text(&'a str),
    Stdin(&'a mut dyn Read),
    TxtFile(&'a Path),
}

pub fn read_input(source: InputSource<'_>, limits: TextLimits) -> Result<RawDocument, ParserError> {
    match source {
        InputSource::Text(content) => {
            document_from_bytes(None, content.as_bytes(), "<text>", SourceType::Text, limits)
        }
        InputSource::Stdin(reader) => {
            let bytes = read_limited(reader, "<stdin>", limits)?;
            document_from_bytes(None, &bytes, "<stdin>", SourceType::Stdin, limits)
        }
        InputSource::TxtFile(path) => {
            let path_display = path.to_string_lossy().into_owned();
            let bytes = read_file_limited(path, &path_display, limits)?;
            let file_name = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned());
            document_from_bytes(
                file_name.as_deref(),
                &bytes,
                &path_display,
                SourceType::Txt,
                limits,
            )
        }
    }
}

pub fn read_txt(path: impl AsRef<Path>) -> Result<RawDocument, ParserError> {
    read_input(InputSource::TxtFile(path.as_ref()), TextLimits::default())
}

pub fn read_txt_bytes(
    file_name: Option<&str>,
    bytes: &[u8],
    source_path: &str,
) -> Result<RawDocument, ParserError> {
    document_from_bytes(
        file_name,
        bytes,
        source_path,
        SourceType::Txt,
        TextLimits::default(),
    )
}

fn read_file_limited(
    path: &Path,
    source: &str,
    limits: TextLimits,
) -> Result<Vec<u8>, ParserError> {
    let mut file = File::open(path).map_err(|error| ParserError::Io {
        path: source.to_owned(),
        kind: error.kind().into(),
    })?;
    read_limited(&mut file, source, limits)
}

fn read_limited(
    reader: &mut dyn Read,
    source: &str,
    limits: TextLimits,
) -> Result<Vec<u8>, ParserError> {
    let mut bytes = Vec::new();
    let read_limit = limits.max_bytes.saturating_add(1) as u64;
    reader
        .take(read_limit)
        .read_to_end(&mut bytes)
        .map_err(|error| ParserError::Io {
            path: source.to_owned(),
            kind: error.kind().into(),
        })?;

    if bytes.len() > limits.max_bytes {
        return Err(ParserError::InputTooLarge {
            source: source.to_owned(),
            limit: limits.max_bytes,
            actual: bytes.len(),
        });
    }

    Ok(bytes)
}

fn document_from_bytes(
    file_name: Option<&str>,
    bytes: &[u8],
    source_path: &str,
    source_type: SourceType,
    limits: TextLimits,
) -> Result<RawDocument, ParserError> {
    if bytes.len() > limits.max_bytes {
        return Err(ParserError::InputTooLarge {
            source: source_path.to_owned(),
            limit: limits.max_bytes,
            actual: bytes.len(),
        });
    }

    let ranges = line_ranges(bytes);
    for (index, (start, end)) in ranges.iter().copied().enumerate() {
        if end - start > limits.max_line_bytes {
            return Err(ParserError::LineTooLong {
                source: source_path.to_owned(),
                line: index + 1,
                limit: limits.max_line_bytes,
                actual: end - start,
            });
        }
    }

    let text = String::from_utf8(bytes.to_vec()).map_err(|error| ParserError::InvalidUtf8 {
        path: source_path.to_owned(),
        valid_up_to: error.utf8_error().valid_up_to(),
    })?;

    let blocks = ranges
        .into_iter()
        .enumerate()
        .map(|(index, (start, end))| RawBlock {
            id: format!("block-{}", index + 1),
            value: RawValue::text(&text[start..end]),
            location: SourceLocation {
                line: Some(index + 1),
                byte_start: Some(start),
                byte_end: Some(end),
                ..SourceLocation::default()
            },
        })
        .collect();

    Ok(RawDocument::new(
        "txt-document",
        SourceMetadata {
            source_type,
            file_name: file_name.map(str::to_owned),
            mime_type: Some("text/plain".to_owned()),
            size_bytes: Some(bytes.len() as u64),
        },
        blocks,
    ))
}

fn line_ranges(bytes: &[u8]) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut line_start = 0;
    let mut index = 0;

    while index < bytes.len() {
        match bytes[index] {
            b'\n' => {
                ranges.push((line_start, index));
                index += 1;
                line_start = index;
            }
            b'\r' => {
                ranges.push((line_start, index));
                index += 1;
                if bytes.get(index) == Some(&b'\n') {
                    index += 1;
                }
                line_start = index;
            }
            _ => index += 1,
        }
    }

    if line_start < bytes.len() {
        ranges.push((line_start, bytes.len()));
    }

    ranges
}

pub fn formats_ready() -> bool {
    true
}

#[cfg(test)]
mod tests {
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
        let text_document = read_input(InputSource::Text(content), TextLimits::default())
            .expect("text should be read");
        let mut stdin = Cursor::new(content.as_bytes());
        let stdin_document = read_input(InputSource::Stdin(&mut stdin), TextLimits::default())
            .expect("stdin should be read");

        assert_eq!(text_document.blocks, stdin_document.blocks);
        assert_eq!(text_document.source.source_type, SourceType::Text);
        assert_eq!(stdin_document.source.source_type, SourceType::Stdin);
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
    fn empty_formats_test() {
        assert!(formats_ready());
    }
}

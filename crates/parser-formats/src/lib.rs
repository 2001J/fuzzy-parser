use parser_core::{
    ParserError, RawBlock, RawDocument, RawValue, SourceLocation, SourceMetadata, SourceType,
};
use std::{fs, path::Path};

pub fn read_txt(path: impl AsRef<Path>) -> Result<RawDocument, ParserError> {
    let path = path.as_ref();
    let path_display = path.to_string_lossy().into_owned();
    let bytes = fs::read(path).map_err(|error| ParserError::Io {
        path: path_display,
        kind: error.kind().into(),
    })?;
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned());

    read_txt_bytes(file_name.as_deref(), &bytes, &path.to_string_lossy())
}

pub fn read_txt_bytes(
    file_name: Option<&str>,
    bytes: &[u8],
    source_path: &str,
) -> Result<RawDocument, ParserError> {
    let text = String::from_utf8(bytes.to_vec()).map_err(|error| ParserError::InvalidUtf8 {
        path: source_path.to_owned(),
        valid_up_to: error.utf8_error().valid_up_to(),
    })?;

    let blocks = line_ranges(bytes)
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
            source_type: SourceType::Txt,
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
    use std::path::PathBuf;

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

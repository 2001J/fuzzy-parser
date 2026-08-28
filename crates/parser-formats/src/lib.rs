use calamine::{Data, Reader, Xlsx, XlsxError, open_workbook, open_workbook_from_rs};
use parser_core::{
    ParserError, RawBlock, RawDocument, RawValue, SourceLocation, SourceMetadata, SourceType,
};
use std::{
    fs,
    fs::File,
    io::{Cursor, Read, Seek},
    path::Path,
};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CsvDelimiter {
    Comma,
    Semicolon,
    Tab,
    Pipe,
}

impl CsvDelimiter {
    const CANDIDATES: [Self; 4] = [Self::Comma, Self::Semicolon, Self::Tab, Self::Pipe];

    fn byte(self) -> u8 {
        match self {
            Self::Comma => b',',
            Self::Semicolon => b';',
            Self::Tab => b'\t',
            Self::Pipe => b'|',
        }
    }

    pub const fn symbol(self) -> &'static str {
        match self {
            Self::Comma => ",",
            Self::Semicolon => ";",
            Self::Tab => "\\t",
            Self::Pipe => "|",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CsvOptions {
    pub delimiter: Option<CsvDelimiter>,
}

impl CsvOptions {
    pub const fn with_delimiter(delimiter: CsvDelimiter) -> Self {
        Self {
            delimiter: Some(delimiter),
        }
    }
}

pub fn read_csv(path: impl AsRef<Path>) -> Result<RawDocument, ParserError> {
    read_csv_with_options(path, CsvOptions::default())
}

pub fn read_csv_with_options(
    path: impl AsRef<Path>,
    options: CsvOptions,
) -> Result<RawDocument, ParserError> {
    let path = path.as_ref();
    let path_display = path.to_string_lossy().into_owned();
    let bytes = fs::read(path).map_err(|error| ParserError::Io {
        path: path_display.clone(),
        kind: error.kind().into(),
    })?;
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned());

    read_csv_bytes(file_name.as_deref(), &bytes, &path_display, options)
}

pub fn read_csv_bytes(
    file_name: Option<&str>,
    bytes: &[u8],
    source_path: &str,
    options: CsvOptions,
) -> Result<RawDocument, ParserError> {
    let (delimiter, records) = match options.delimiter {
        Some(delimiter) => (delimiter, parse_csv_records(bytes, source_path, delimiter)?),
        None => detect_delimiter(bytes, source_path)?,
    };

    let blocks = records
        .into_iter()
        .enumerate()
        .flat_map(|(row_index, row)| {
            row.into_iter()
                .enumerate()
                .map(move |(column_index, value)| RawBlock {
                    id: format!("row-{}-column-{}", row_index + 1, column_index + 1),
                    value: RawValue::text(value),
                    location: SourceLocation {
                        row: Some(row_index + 1),
                        column: Some(column_index + 1),
                        ..SourceLocation::default()
                    },
                })
        })
        .collect();

    Ok(RawDocument::new(
        "csv-document",
        SourceMetadata {
            source_type: SourceType::Csv,
            file_name: file_name.map(str::to_owned),
            mime_type: Some("text/csv".to_owned()),
            size_bytes: Some(bytes.len() as u64),
            delimiter: Some(delimiter.symbol().to_owned()),
        },
        blocks,
    ))
}

type DelimiterScore = (usize, usize, usize, usize);
type DelimiterCandidate = (DelimiterScore, CsvDelimiter, Vec<Vec<String>>);

fn detect_delimiter(
    bytes: &[u8],
    source_path: &str,
) -> Result<(CsvDelimiter, Vec<Vec<String>>), ParserError> {
    let mut best: Option<DelimiterCandidate> = None;
    let mut first_error = None;

    for (index, delimiter) in CsvDelimiter::CANDIDATES.into_iter().enumerate() {
        match parse_csv_records(bytes, source_path, delimiter) {
            Ok(records) => {
                let score = delimiter_score(&records, index);
                let should_replace = best
                    .as_ref()
                    .map(|(best_score, _, _)| score > *best_score)
                    .unwrap_or(true);
                if should_replace {
                    best = Some((score, delimiter, records));
                }
            }
            Err(error) => {
                if first_error.is_none() {
                    first_error = Some(error);
                }
            }
        }
    }

    if let Some((_, delimiter, records)) = best {
        Ok((delimiter, records))
    } else if let Some(error) = first_error {
        Err(error)
    } else {
        Err(ParserError::InvalidCsv {
            path: source_path.to_owned(),
            record: None,
            message: "no supported delimiter could parse the input".to_owned(),
        })
    }
}

fn delimiter_score(records: &[Vec<String>], candidate_index: usize) -> DelimiterScore {
    let multi_field_records = records.iter().filter(|record| record.len() > 1).count();
    let total_fields = records.iter().map(Vec::len).sum();
    let widest_record = records.iter().map(Vec::len).max().unwrap_or(0);
    let consistent_records = records
        .iter()
        .filter(|record| record.len() == widest_record)
        .count();

    (
        multi_field_records,
        consistent_records,
        total_fields,
        usize::MAX - candidate_index,
    )
}

fn parse_csv_records(
    bytes: &[u8],
    source_path: &str,
    delimiter: CsvDelimiter,
) -> Result<Vec<Vec<String>>, ParserError> {
    validate_csv_quotes(bytes, source_path, delimiter.byte())?;

    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .delimiter(delimiter.byte())
        .from_reader(bytes);
    let mut records = Vec::new();

    for result in reader.records() {
        let record = result.map_err(|error| ParserError::InvalidCsv {
            path: source_path.to_owned(),
            record: Some(records.len() + 1),
            message: error.to_string(),
        })?;
        records.push(record.iter().map(str::to_owned).collect());
    }

    Ok(records)
}

fn validate_csv_quotes(bytes: &[u8], source_path: &str, delimiter: u8) -> Result<(), ParserError> {
    let mut in_quotes = false;
    let mut field_start = true;
    let mut index = 0;

    while index < bytes.len() {
        let byte = bytes[index];
        if in_quotes {
            if byte == b'"' {
                if bytes.get(index + 1) == Some(&b'"') {
                    index += 2;
                } else {
                    in_quotes = false;
                    field_start = false;
                    index += 1;
                }
            } else {
                index += 1;
            }
            continue;
        }

        match byte {
            b'"' if field_start => {
                in_quotes = true;
                field_start = false;
                index += 1;
            }
            b'"' => {
                return Err(ParserError::InvalidCsv {
                    path: source_path.to_owned(),
                    record: None,
                    message: "unexpected quote in unquoted field".to_owned(),
                });
            }
            value if value == delimiter => {
                field_start = true;
                index += 1;
            }
            b'\n' => {
                field_start = true;
                index += 1;
            }
            b'\r' => {
                field_start = true;
                index += 1;
                if bytes.get(index) == Some(&b'\n') {
                    index += 1;
                }
            }
            _ => {
                field_start = false;
                index += 1;
            }
        }
    }

    if in_quotes {
        return Err(ParserError::InvalidCsv {
            path: source_path.to_owned(),
            record: None,
            message: "unterminated quoted field".to_owned(),
        });
    }

    Ok(())
}

pub fn read_xlsx(path: impl AsRef<Path>) -> Result<RawDocument, ParserError> {
    let path = path.as_ref();
    let path_display = path.to_string_lossy().into_owned();
    let size_bytes = fs::metadata(path)
        .map_err(|error| ParserError::Io {
            path: path_display.clone(),
            kind: error.kind().into(),
        })?
        .len();
    let workbook: Xlsx<_> =
        open_workbook(path).map_err(|error| xlsx_error(Some(&path_display), None, error))?;
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned());
    extract_xlsx(
        workbook,
        file_name.as_deref(),
        size_bytes,
        Some(&path_display),
    )
}

/// Reads borrowed XLSX bytes without filesystem or network access.
///
/// `file_name` is optional caller metadata, never a path to open. Stored cell
/// values are read without evaluating formulas, macros or external links.
/// Invalid input returns `InvalidXlsx` with an empty `path` and a generic
/// message; filename and workbook contents are not copied into diagnostics.
/// This API does not yet impose workbook/decompression/cell limits.
pub fn read_xlsx_bytes(file_name: Option<&str>, bytes: &[u8]) -> Result<RawDocument, ParserError> {
    let workbook: Xlsx<_> =
        open_workbook_from_rs(Cursor::new(bytes)).map_err(|error| xlsx_error(None, None, error))?;
    extract_xlsx(workbook, file_name, bytes.len() as u64, None)
}

fn extract_xlsx<RS: Read + Seek>(
    mut workbook: Xlsx<RS>,
    file_name: Option<&str>,
    size_bytes: u64,
    source_path: Option<&str>,
) -> Result<RawDocument, ParserError> {
    let mut blocks = Vec::new();
    for (sheet_index, sheet_name) in workbook.sheet_names().into_iter().enumerate() {
        let _merged_regions = workbook
            .merge_cells_by_sheet_name(&sheet_name)
            .map_err(|error| xlsx_error(source_path, Some(&sheet_name), error))?;
        let range = workbook
            .worksheet_range(&sheet_name)
            .map_err(|error| xlsx_error(source_path, Some(&sheet_name), error))?;
        let Some((start_row, start_column)) = range.start() else {
            continue;
        };

        for (row_offset, row) in range.rows().enumerate() {
            for (column_offset, cell) in row.iter().enumerate() {
                blocks.push(RawBlock {
                    id: format!(
                        "sheet-{}-row-{}-column-{}",
                        sheet_index + 1,
                        start_row as usize + row_offset + 1,
                        start_column as usize + column_offset + 1
                    ),
                    value: raw_xlsx_value(cell),
                    location: SourceLocation {
                        row: Some(start_row as usize + row_offset + 1),
                        column: Some(start_column as usize + column_offset + 1),
                        sheet: Some(sheet_name.clone()),
                        ..SourceLocation::default()
                    },
                });
            }
        }
    }

    Ok(RawDocument::new(
        "xlsx-document",
        SourceMetadata {
            source_type: SourceType::Xlsx,
            file_name: file_name.map(str::to_owned),
            mime_type: Some(
                "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet".to_owned(),
            ),
            size_bytes: Some(size_bytes),
            delimiter: None,
        },
        blocks,
    ))
}

fn xlsx_error(path: Option<&str>, sheet: Option<&str>, error: XlsxError) -> ParserError {
    let message = match (path, sheet) {
        (Some(_), Some(sheet)) => format!("sheet {sheet}: {error:?}"),
        (Some(_), None) => format!("{error:?}"),
        (None, _) => "could not read XLSX workbook".to_owned(),
    };
    ParserError::InvalidXlsx {
        path: path.unwrap_or_default().to_owned(),
        message,
    }
}

fn raw_xlsx_value(cell: &Data) -> RawValue {
    match cell {
        Data::Int(value) => RawValue::Integer(*value),
        Data::Float(value) => RawValue::Decimal(*value),
        Data::String(value) => RawValue::Text(value.clone()),
        Data::Bool(value) => RawValue::Boolean(*value),
        Data::DateTime(value) => RawValue::DateTime(value.as_f64()),
        Data::DateTimeIso(value) => RawValue::DateTimeText(value.clone()),
        Data::DurationIso(value) => RawValue::Duration(value.clone()),
        Data::Error(value) => RawValue::Error(value.to_string()),
        Data::Empty => RawValue::Null,
    }
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
            delimiter: None,
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
#[path = "../tests/unit/mod.rs"]
mod tests;

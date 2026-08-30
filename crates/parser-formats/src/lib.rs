use calamine::{Data, Reader, Xlsx, XlsxError, open_workbook, open_workbook_from_rs};
use parser_core::{
    Failure, ParsePlan, ParseResponse, ParserError, RawBlock, RawDocument, RawValue,
    SourceLocation, SourceMetadata, SourceType, TableByteSpan, TableInventory, TableInventoryRow,
    TableInventorySheet, TableSelectionOptions, UnsupportedTableMetadata,
};
use std::{
    fs,
    io::{Cursor, Read, Seek},
    path::Path,
};

mod file_validation;
pub use file_validation::{
    EmptyFilePolicy, FileFormat, FileValidationOptions, ValidatedFile, open_validated_file,
};

pub const DEFAULT_MAX_TEXT_BYTES: usize = 1024 * 1024;
pub const DEFAULT_MAX_LINE_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnsupportedMetadata {
    MergedRegion {
        start_row: usize,
        start_column: usize,
        end_row: usize,
        end_column: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedRow {
    pub source_row: usize,
    pub block_indices: Vec<usize>,
    pub blank: bool,
    pub byte_span: Option<TableByteSpan>,
    pub line_start: Option<usize>,
    pub line_end: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedSheet {
    pub original_index: usize,
    pub name: Option<String>,
    pub rows: Vec<ExtractedRow>,
    pub unsupported_metadata: Vec<UnsupportedMetadata>,
}

/// Extraction companion for opt-in table selection. It intentionally has no
/// serde implementation; the canonical document remains the wire source model.
#[derive(Debug, Clone, PartialEq)]
pub struct ExtractedTable {
    pub document: RawDocument,
    pub manifest: Vec<ExtractedSheet>,
}

impl ExtractedTable {
    fn inventory(&self) -> TableInventory {
        TableInventory {
            source_type: self.document.source.source_type.clone(),
            sheets: self
                .manifest
                .iter()
                .map(|sheet| TableInventorySheet {
                    original_index: sheet.original_index,
                    name: sheet.name.clone(),
                    rows: sheet
                        .rows
                        .iter()
                        .map(|row| TableInventoryRow {
                            source_row: row.source_row,
                            block_indices: row.block_indices.clone(),
                            blank: row.blank,
                            byte_span: row.byte_span,
                            line_start: row.line_start,
                            line_end: row.line_end,
                        })
                        .collect(),
                    unsupported_metadata: sheet
                        .unsupported_metadata
                        .iter()
                        .map(|metadata| match metadata {
                            UnsupportedMetadata::MergedRegion {
                                start_row,
                                start_column,
                                end_row,
                                end_column,
                            } => UnsupportedTableMetadata::MergedRegion {
                                start_row: *start_row,
                                start_column: *start_column,
                                end_row: *end_row,
                                end_column: *end_column,
                            },
                        })
                        .collect(),
                })
                .collect(),
        }
    }
}

pub fn parse_extracted_table_with_plan(
    table: &ExtractedTable,
    plan: &ParsePlan,
    options: &TableSelectionOptions,
) -> Result<ParseResponse, Failure> {
    parser_core::parse_document_with_plan_and_table_selection(
        &table.document,
        plan,
        &table.inventory(),
        options,
    )
}

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

/// TXT uses one byte limit for both validation and bounded extraction.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct TxtOptions {
    pub limits: TextLimits,
    pub empty_policy: EmptyFilePolicy,
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
        InputSource::TxtFile(path) => read_txt_with_options(
            path,
            TxtOptions {
                limits,
                empty_policy: EmptyFilePolicy::Accept,
            },
        ),
    }
}

pub fn read_txt(path: impl AsRef<Path>) -> Result<RawDocument, ParserError> {
    read_txt_with_options(path, TxtOptions::default())
}

/// Validate a `.txt` path and extract from that same handle, following symlinks.
pub fn read_txt_with_options(
    path: impl AsRef<Path>,
    options: TxtOptions,
) -> Result<RawDocument, ParserError> {
    let path = path.as_ref();
    let input = open_validated_file(
        path,
        &FileValidationOptions {
            enabled_formats: vec![FileFormat::Txt],
            max_bytes: options.limits.max_bytes,
            empty_policy: options.empty_policy,
        },
    )?;
    read_validated_txt(input, path, options)
}

fn read_validated_txt(
    input: ValidatedFile,
    path: &Path,
    options: TxtOptions,
) -> Result<RawDocument, ParserError> {
    let source = path.to_string_lossy();
    let bytes = read_limited(&mut input.into_file(), &source, options.limits)?;
    file_validation::check_empty(bytes.len() as u64, &source, options.empty_policy)?;
    let file_name = path.file_name().map(|name| name.to_string_lossy());
    document_from_bytes(
        file_name.as_deref(),
        &bytes,
        &source,
        SourceType::Txt,
        options.limits,
    )
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

pub fn read_csv_table(path: impl AsRef<Path>) -> Result<ExtractedTable, ParserError> {
    read_csv_table_with_options(path, CsvOptions::default())
}

pub fn read_csv_table_with_options(
    path: impl AsRef<Path>,
    options: CsvOptions,
) -> Result<ExtractedTable, ParserError> {
    let path = path.as_ref();
    let path_display = path.to_string_lossy().into_owned();
    let bytes = fs::read(path).map_err(|error| ParserError::Io {
        path: path_display.clone(),
        kind: error.kind().into(),
    })?;
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned());
    read_csv_table_bytes(file_name.as_deref(), &bytes, &path_display, options)
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

pub fn read_csv_table_bytes(
    file_name: Option<&str>,
    bytes: &[u8],
    source_path: &str,
    options: CsvOptions,
) -> Result<ExtractedTable, ParserError> {
    let (delimiter, records) = match options.delimiter {
        Some(delimiter) => (delimiter, parse_csv_records(bytes, source_path, delimiter)?),
        None => detect_delimiter(bytes, source_path)?,
    };
    let logical_rows = csv_logical_rows(bytes, delimiter.byte());
    let mut records = records.into_iter();
    let mut blocks = Vec::new();
    let mut rows = Vec::new();
    for row in logical_rows {
        let block_start = blocks.len();
        if !row.blank {
            let record = records.next().ok_or_else(|| ParserError::InvalidCsv {
                path: source_path.to_owned(),
                record: Some(row.source_row),
                message: "CSV row inventory did not match parsed records".to_owned(),
            })?;
            for (column_index, value) in record.into_iter().enumerate() {
                blocks.push(RawBlock {
                    id: format!("row-{}-column-{}", row.source_row, column_index + 1),
                    value: RawValue::text(value),
                    location: SourceLocation {
                        row: Some(row.source_row),
                        column: Some(column_index + 1),
                        ..SourceLocation::default()
                    },
                });
            }
        }
        rows.push(ExtractedRow {
            source_row: row.source_row,
            block_indices: (block_start..blocks.len()).collect(),
            blank: row.blank,
            byte_span: Some(TableByteSpan {
                byte_start: row.byte_start,
                byte_end: row.byte_end,
            }),
            line_start: Some(row.line_start),
            line_end: Some(row.line_end),
        });
    }
    if records.next().is_some() {
        return Err(ParserError::InvalidCsv {
            path: source_path.to_owned(),
            record: None,
            message: "CSV row inventory did not match parsed records".to_owned(),
        });
    }
    Ok(ExtractedTable {
        document: RawDocument::new(
            "csv-document",
            SourceMetadata {
                source_type: SourceType::Csv,
                file_name: file_name.map(str::to_owned),
                mime_type: Some("text/csv".to_owned()),
                size_bytes: Some(bytes.len() as u64),
                delimiter: Some(delimiter.symbol().to_owned()),
            },
            blocks,
        ),
        manifest: vec![ExtractedSheet {
            original_index: 1,
            name: None,
            rows,
            unsupported_metadata: Vec::new(),
        }],
    })
}

#[derive(Debug, Clone, Copy)]
struct CsvLogicalRow {
    source_row: usize,
    byte_start: usize,
    byte_end: usize,
    line_start: usize,
    line_end: usize,
    blank: bool,
}

fn csv_logical_rows(bytes: &[u8], delimiter: u8) -> Vec<CsvLogicalRow> {
    let mut rows = Vec::new();
    let mut row_start = 0;
    let mut row_line_start = 1;
    let mut line = 1;
    let mut in_quotes = false;
    let mut field_start = true;
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'"' if in_quotes && bytes.get(index + 1) == Some(&b'"') => index += 2,
            b'"' if in_quotes => {
                in_quotes = false;
                field_start = false;
                index += 1;
            }
            b'"' if field_start => {
                in_quotes = true;
                field_start = false;
                index += 1;
            }
            b'\r' | b'\n' => {
                let crlf = bytes[index] == b'\r' && bytes.get(index + 1) == Some(&b'\n');
                if in_quotes {
                    line += 1;
                    index += if crlf { 2 } else { 1 };
                    continue;
                }
                rows.push(CsvLogicalRow {
                    source_row: rows.len() + 1,
                    byte_start: row_start,
                    byte_end: index,
                    line_start: row_line_start,
                    line_end: line,
                    blank: row_start == index,
                });
                line += 1;
                index += if crlf { 2 } else { 1 };
                row_start = index;
                row_line_start = line;
                field_start = true;
            }
            byte if !in_quotes && byte == delimiter => {
                field_start = true;
                index += 1;
            }
            _ => {
                field_start = false;
                index += 1;
            }
        }
    }
    if row_start < bytes.len() {
        rows.push(CsvLogicalRow {
            source_row: rows.len() + 1,
            byte_start: row_start,
            byte_end: bytes.len(),
            line_start: row_line_start,
            line_end: line,
            blank: row_start == bytes.len(),
        });
    }
    rows
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

pub fn read_xlsx_table(path: impl AsRef<Path>) -> Result<ExtractedTable, ParserError> {
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
    extract_xlsx_table(
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

pub fn read_xlsx_table_bytes(
    file_name: Option<&str>,
    bytes: &[u8],
) -> Result<ExtractedTable, ParserError> {
    let workbook: Xlsx<_> =
        open_workbook_from_rs(Cursor::new(bytes)).map_err(|error| xlsx_error(None, None, error))?;
    extract_xlsx_table(workbook, file_name, bytes.len() as u64, None)
}

fn extract_xlsx<RS: Read + Seek>(
    workbook: Xlsx<RS>,
    file_name: Option<&str>,
    size_bytes: u64,
    source_path: Option<&str>,
) -> Result<RawDocument, ParserError> {
    extract_xlsx_table(workbook, file_name, size_bytes, source_path).map(|table| table.document)
}

fn extract_xlsx_table<RS: Read + Seek>(
    mut workbook: Xlsx<RS>,
    file_name: Option<&str>,
    size_bytes: u64,
    source_path: Option<&str>,
) -> Result<ExtractedTable, ParserError> {
    let mut blocks = Vec::new();
    let mut manifest = Vec::new();
    for (sheet_index, sheet_name) in workbook.sheet_names().into_iter().enumerate() {
        let merged_regions = workbook
            .merge_cells_by_sheet_name(&sheet_name)
            .map_err(|error| xlsx_error(source_path, Some(&sheet_name), error))?;
        let range = workbook
            .worksheet_range(&sheet_name)
            .map_err(|error| xlsx_error(source_path, Some(&sheet_name), error))?;
        let mut rows = Vec::new();
        if let Some((start_row, start_column)) = range.start() {
            for (row_offset, row) in range.rows().enumerate() {
                let source_row = start_row as usize + row_offset + 1;
                let block_start = blocks.len();
                let blank = row.iter().all(|cell| matches!(cell, Data::Empty));
                for (column_offset, cell) in row.iter().enumerate() {
                    blocks.push(RawBlock {
                        id: format!(
                            "sheet-{}-row-{}-column-{}",
                            sheet_index + 1,
                            source_row,
                            start_column as usize + column_offset + 1
                        ),
                        value: raw_xlsx_value(cell),
                        location: SourceLocation {
                            row: Some(source_row),
                            column: Some(start_column as usize + column_offset + 1),
                            sheet: Some(sheet_name.clone()),
                            ..SourceLocation::default()
                        },
                    });
                }
                rows.push(ExtractedRow {
                    source_row,
                    block_indices: (block_start..blocks.len()).collect(),
                    blank,
                    byte_span: None,
                    line_start: None,
                    line_end: None,
                });
            }
        }
        manifest.push(ExtractedSheet {
            original_index: sheet_index + 1,
            name: Some(sheet_name),
            rows,
            unsupported_metadata: merged_regions
                .into_iter()
                .map(|region| UnsupportedMetadata::MergedRegion {
                    start_row: region.start.0 as usize + 1,
                    start_column: region.start.1 as usize + 1,
                    end_row: region.end.0 as usize + 1,
                    end_column: region.end.1 as usize + 1,
                })
                .collect(),
        });
    }
    Ok(ExtractedTable {
        document: RawDocument::new(
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
        ),
        manifest,
    })
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

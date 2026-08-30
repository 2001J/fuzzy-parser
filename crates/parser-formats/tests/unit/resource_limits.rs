use super::txt_fixtures::TempDirectory;
use super::*;
use parser_core::{
    FailureKind, ParseContent, ParseLimits, ParsePlan, ResourceLimitKind, TableSelectionOptions,
};
use std::fs;

fn assert_resource(error: ParserError, resource: ResourceLimitKind, limit: u64, actual: u64) {
    assert_eq!(
        error,
        ParserError::ResourceLimit {
            resource,
            limit,
            actual,
        }
    );
}

fn csv_options(bytes: u64, rows: usize, cells: usize) -> CsvOptions {
    CsvOptions {
        delimiter: Some(CsvDelimiter::Comma),
        limits: CsvLimits {
            max_bytes: bytes,
            max_rows: rows,
            max_cells: cells,
        },
    }
}

#[test]
fn csv_file_byte_and_table_paths_share_exact_and_one_over_limits() {
    let bytes = b"a,b\nc,d";
    let exact = csv_options(bytes.len() as u64, 2, 4);
    assert!(read_csv_bytes(None, bytes, "rows.csv", exact).is_ok());
    assert!(read_csv_table_bytes(None, bytes, "rows.csv", exact).is_ok());

    let directory = TempDirectory::new();
    let path = directory.0.join("rows.csv");
    fs::write(&path, bytes).unwrap();
    assert!(read_csv_with_options(&path, exact).is_ok());
    assert!(read_csv_table_with_options(&path, exact).is_ok());

    let byte_one_over = csv_options(bytes.len() as u64 - 1, 2, 4);
    for error in [
        read_csv_bytes(None, bytes, "rows.csv", byte_one_over).unwrap_err(),
        read_csv_table_bytes(None, bytes, "rows.csv", byte_one_over).unwrap_err(),
        read_csv_with_options(&path, byte_one_over).unwrap_err(),
        read_csv_table_with_options(&path, byte_one_over).unwrap_err(),
    ] {
        assert_resource(
            error,
            ResourceLimitKind::CsvBytes,
            bytes.len() as u64 - 1,
            bytes.len() as u64,
        );
    }

    for (options, resource, limit, actual) in [
        (
            csv_options(bytes.len() as u64, 1, 4),
            ResourceLimitKind::CsvRows,
            1,
            2,
        ),
        (
            csv_options(bytes.len() as u64, 2, 3),
            ResourceLimitKind::CsvCells,
            3,
            4,
        ),
    ] {
        for error in [
            read_csv_bytes(None, bytes, "rows.csv", options).unwrap_err(),
            read_csv_table_bytes(None, bytes, "rows.csv", options).unwrap_err(),
        ] {
            assert_resource(error, resource, limit, actual);
        }
    }
}

#[test]
fn csv_row_limit_counts_blank_logical_rows_without_dropping_exact_input() {
    let bytes = b"a,b\n\nc,d\n";
    let exact = csv_options(bytes.len() as u64, 3, 4);
    assert!(read_csv_bytes(None, bytes, "blank.csv", exact).is_ok());
    let table = read_csv_table_bytes(None, bytes, "blank.csv", exact).unwrap();
    assert_eq!(table.manifest[0].rows.len(), 3);

    let one_over = csv_options(bytes.len() as u64, 2, 4);
    for error in [
        read_csv_bytes(None, bytes, "blank.csv", one_over).unwrap_err(),
        read_csv_table_bytes(None, bytes, "blank.csv", one_over).unwrap_err(),
    ] {
        assert_resource(error, ResourceLimitKind::CsvRows, 2, 3);
    }
}

fn sample_xlsx() -> (&'static [u8], std::path::PathBuf) {
    (
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/xlsx/sample.xlsx"
        )),
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/xlsx/sample.xlsx"),
    )
}

fn xlsx_limits(bytes: u64, sheets: usize, cells: usize) -> XlsxLimits {
    XlsxLimits {
        max_bytes: bytes,
        max_sheets: sheets,
        max_cells: cells,
    }
}

#[test]
fn xlsx_file_byte_document_and_table_paths_share_one_limit_contract() {
    let (bytes, path) = sample_xlsx();
    let exact = xlsx_limits(bytes.len() as u64, 1, 12);
    let byte_table = read_xlsx_table_bytes_with_limits(Some("sample.xlsx"), bytes, exact).unwrap();
    assert_eq!(byte_table.manifest.len(), 1);
    assert_eq!(byte_table.document.blocks.len(), 12);
    assert_eq!(
        read_xlsx_bytes_with_limits(Some("sample.xlsx"), bytes, exact).unwrap(),
        byte_table.document
    );
    assert_eq!(
        read_xlsx_table_with_limits(&path, exact).unwrap(),
        byte_table
    );
    assert_eq!(
        read_xlsx_with_limits(&path, exact).unwrap(),
        byte_table.document
    );

    let byte_one_over = xlsx_limits(bytes.len() as u64 - 1, 1, 12);
    for error in [
        read_xlsx_bytes_with_limits(None, bytes, byte_one_over).unwrap_err(),
        read_xlsx_table_bytes_with_limits(None, bytes, byte_one_over).unwrap_err(),
        read_xlsx_with_limits(&path, byte_one_over).unwrap_err(),
        read_xlsx_table_with_limits(&path, byte_one_over).unwrap_err(),
    ] {
        assert_resource(
            error,
            ResourceLimitKind::XlsxBytes,
            bytes.len() as u64 - 1,
            bytes.len() as u64,
        );
    }

    for (limits, resource, limit, actual) in [
        (
            xlsx_limits(bytes.len() as u64, 0, 12),
            ResourceLimitKind::XlsxSheets,
            0,
            1,
        ),
        (
            xlsx_limits(bytes.len() as u64, 1, 11),
            ResourceLimitKind::XlsxCells,
            11,
            12,
        ),
    ] {
        for error in [
            read_xlsx_bytes_with_limits(None, bytes, limits).unwrap_err(),
            read_xlsx_table_bytes_with_limits(None, bytes, limits).unwrap_err(),
        ] {
            assert_resource(error, resource, limit, actual);
        }
    }
}

#[test]
fn xlsx_sheet_limit_counts_empty_sheets_and_byte_limit_precedes_corrupt_decode() {
    let bytes = super::table_selection_xlsx_bytes();
    let failure =
        read_xlsx_bytes_with_limits(None, &bytes, xlsx_limits(bytes.len() as u64, 2, usize::MAX))
            .unwrap_err();
    assert_resource(failure, ResourceLimitKind::XlsxSheets, 2, 3);

    let corrupt = b"not an xlsx";
    assert!(matches!(
        read_xlsx_table_bytes_with_limits(
            None,
            corrupt,
            xlsx_limits(corrupt.len() as u64, usize::MAX, usize::MAX)
        ),
        Err(ParserError::InvalidXlsx { .. })
    ));
    assert_resource(
        read_xlsx_table_bytes_with_limits(
            None,
            corrupt,
            xlsx_limits(corrupt.len() as u64 - 1, usize::MAX, usize::MAX),
        )
        .unwrap_err(),
        ResourceLimitKind::XlsxBytes,
        corrupt.len() as u64 - 1,
        corrupt.len() as u64,
    );
}

#[test]
fn selected_table_record_and_response_limits_use_parsed_output_not_raw_cells() {
    let (bytes, _) = sample_xlsx();
    let table = read_xlsx_table_bytes(Some("sample.xlsx"), bytes).unwrap();
    let plan = ParsePlan::new(Vec::new(), None);
    let options = TableSelectionOptions::default();
    let response = parse_extracted_table_with_plan(&table, &plan, &options).unwrap();
    let records = match &response.content {
        ParseContent::Table { sheets } => sheets.iter().map(|sheet| sheet.records.len()).sum(),
        ParseContent::Text { .. } => panic!("expected table response"),
    };
    assert!(records < table.document.blocks.len());
    let response_bytes = match parse_extracted_table_with_plan_and_limits(
        &table,
        &plan,
        &options,
        ParseLimits {
            max_records: usize::MAX,
            max_response_bytes: 0,
        },
    )
    .unwrap_err()
    .kind
    {
        FailureKind::ResourceLimit {
            resource: ResourceLimitKind::ResponseBytes,
            actual,
            ..
        } => actual as usize,
        failure => panic!("unexpected size probe failure: {failure:?}"),
    };
    let exact = ParseLimits {
        max_records: records,
        max_response_bytes: response_bytes,
    };
    assert_eq!(
        parse_extracted_table_with_plan_and_limits(&table, &plan, &options, exact).unwrap(),
        response
    );

    let record_failure = parse_extracted_table_with_plan_and_limits(
        &table,
        &plan,
        &options,
        ParseLimits {
            max_records: records - 1,
            ..exact
        },
    )
    .unwrap_err();
    assert_eq!(
        record_failure.kind,
        FailureKind::ResourceLimit {
            resource: ResourceLimitKind::Records,
            limit: (records - 1) as u64,
            actual: records as u64,
        }
    );

    let response_failure = parse_extracted_table_with_plan_and_limits(
        &table,
        &plan,
        &options,
        ParseLimits {
            max_response_bytes: response_bytes - 1,
            ..exact
        },
    )
    .unwrap_err();
    assert_eq!(
        response_failure.kind,
        FailureKind::ResourceLimit {
            resource: ResourceLimitKind::ResponseBytes,
            limit: (response_bytes - 1) as u64,
            actual: response_bytes as u64,
        }
    );
}

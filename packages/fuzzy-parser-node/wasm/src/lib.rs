use parser_core::{
    DiagnosticsMode, Failure, FailureKind, HeaderSelection, InclusiveRowRange, ParseLimits,
    RowSelection, SheetSelection, SheetSelector, TableSelectionOptions,
};
use parser_formats::{
    CsvOptions, InputSource, TextLimits, parse_extracted_table_with_plan_and_limits,
    read_csv_bytes, read_csv_table_bytes, read_input, read_txt_bytes, read_xlsx_bytes,
    read_xlsx_table_bytes,
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

const ADAPTER_VERSION: &str = env!("CARGO_PKG_VERSION");
const SOURCE_IDENTITY: &str = env!("FUZZY_PARSER_SOURCE_IDENTITY");

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = globalThis, js_name = __fuzzyParserParseEntry)]
    fn fuzzy_parser_parse_entry();
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeIdentity<'a> {
    adapter_version: &'a str,
    contract_version: &'a str,
    parser_version: &'a str,
    schema_version: &'a str,
    source_identity: &'a str,
    wasm_bindgen_version: &'a str,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum BoundaryEnvelope {
    Success { json: String },
    ParserFailure { json: String },
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
struct AdapterOptions {
    table_selection: Option<TableSelectionInput>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
struct TableSelectionInput {
    header: HeaderInput,
    include_rows: Vec<RowRangeInput>,
    exclude_rows: Vec<RowRangeInput>,
    sheets: SheetInput,
}

#[derive(Debug, Default, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
enum HeaderInput {
    #[default]
    Automatic,
    None,
    Row {
        row: usize,
    },
    SchemaSearch {
        #[serde(rename = "maxRows")]
        max_rows: usize,
    },
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RowRangeInput {
    start: usize,
    end: usize,
}

#[derive(Debug, Default, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
enum SheetInput {
    #[default]
    All,
    Selected {
        sheets: Vec<SheetSelectorInput>,
    },
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum SheetSelectorInput {
    Name { name: String },
    Index { index: usize },
}

impl From<TableSelectionInput> for TableSelectionOptions {
    fn from(value: TableSelectionInput) -> Self {
        let header = match value.header {
            HeaderInput::Automatic => HeaderSelection::Automatic,
            HeaderInput::None => HeaderSelection::None,
            HeaderInput::Row { row } => HeaderSelection::Row(row),
            HeaderInput::SchemaSearch { max_rows } => {
                HeaderSelection::SchemaSearch { max_rows }
            }
        };
        let range = |value: RowRangeInput| InclusiveRowRange::new(value.start, value.end);
        let sheets = match value.sheets {
            SheetInput::All => SheetSelection::All,
            SheetInput::Selected { sheets } => SheetSelection::Selected(
                sheets
                    .into_iter()
                    .map(|selector| match selector {
                        SheetSelectorInput::Name { name } => SheetSelector::Name(name),
                        SheetSelectorInput::Index { index } => SheetSelector::Index(index),
                    })
                    .collect(),
            ),
        };
        Self {
            header,
            rows: RowSelection {
                include: value.include_rows.into_iter().map(range).collect(),
                exclude: value.exclude_rows.into_iter().map(range).collect(),
            },
            sheets,
        }
    }
}

fn encode_envelope(envelope: BoundaryEnvelope) -> String {
    serde_json::to_string(&envelope).unwrap_or_else(|_| {
        r#"{"kind":"parser_failure","json":"{\"error\":{\"error_contract_version\":\"0.1\",\"code\":\"output_serialization_error\",\"target\":\"parse_result\"},\"message\":\"could not serialize parse result\"}"#
            .to_owned()
    })
}

fn parser_failure(error: Failure) -> String {
    let report = error.report(DiagnosticsMode::Safe);
    match serde_json::to_string(&report) {
        Ok(json) => encode_envelope(BoundaryEnvelope::ParserFailure { json }),
        Err(_) => encode_envelope(BoundaryEnvelope::ParserFailure {
            json: r#"{"error":{"error_contract_version":"0.1","code":"output_serialization_error","target":"parse_result"},"message":"could not serialize parse result"}"#.to_owned(),
        }),
    }
}

fn parse(
    format: &str,
    bytes: &[u8],
    filename: Option<&str>,
    schema_json: &str,
    options_json: Option<&str>,
) -> Result<parser_core::ParseResponse, Failure> {
    let schema = parser_schema::decode_execution_schema_with_limits(
        schema_json,
        parser_schema::SchemaLimits::default(),
    )?;
    let plan = parser_schema::compile_schema(&schema)?;
    let options = match options_json {
        Some(value) => serde_json::from_str::<AdapterOptions>(value)
            .map_err(|_| Failure::new(FailureKind::SchemaOptionUnsupported))?,
        None => AdapterOptions::default(),
    };

    if let Some(selection) = options.table_selection {
        let table = match format {
            "csv" => read_csv_table_bytes(filename, bytes, "<bytes>", CsvOptions::default()),
            "xlsx" => read_xlsx_table_bytes(filename, bytes),
            _ => {
                return Err(Failure::new(FailureKind::TableSelection {
                    reason: parser_core::TableSelectionReason::UnsupportedSource,
                }));
            }
        }
        .map_err(|error| Failure::from(&error))?;
        fuzzy_parser_parse_entry();
        return parse_extracted_table_with_plan_and_limits(
            &table,
            &plan,
            &selection.into(),
            ParseLimits::default(),
        );
    }

    let document = match format {
        "text" => std::str::from_utf8(bytes)
            .map_err(|error| parser_core::ParserError::InvalidUtf8 {
                path: "<bytes>".to_owned(),
                valid_up_to: error.valid_up_to(),
            })
            .and_then(|text| read_input(InputSource::Text(text), TextLimits::default())),
        "txt" => read_txt_bytes(filename, bytes, "<bytes>"),
        "csv" => read_csv_bytes(filename, bytes, "<bytes>", CsvOptions::default()),
        "xlsx" => read_xlsx_bytes(filename, bytes),
        _ => Err(parser_core::ParserError::UnsupportedInput {
            source_type: format.to_owned(),
        }),
    }
    .map_err(|error| Failure::from(&error))?;
    fuzzy_parser_parse_entry();
    parser_core::parse_document_with_plan_with_limits(&document, &plan, ParseLimits::default())
}

#[wasm_bindgen]
pub fn runtime_identity_json() -> String {
    serde_json::to_string(&RuntimeIdentity {
        adapter_version: ADAPTER_VERSION,
        contract_version: parser_core::CONTRACT_VERSION,
        parser_version: env!("CARGO_PKG_VERSION"),
        schema_version: parser_schema::SCHEMA_VERSION,
        source_identity: SOURCE_IDENTITY,
        wasm_bindgen_version: "0.2.115",
    })
    .expect("fixed runtime identity serializes")
}

#[wasm_bindgen]
pub fn parse_bytes_json(
    format: String,
    bytes: Vec<u8>,
    filename: Option<String>,
    schema: String,
    options: Option<String>,
) -> String {
    match parse(
        &format,
        &bytes,
        filename.as_deref(),
        &schema,
        options.as_deref(),
    ) {
        Ok(response) => match serde_json::to_string(&response) {
            Ok(json) => encode_envelope(BoundaryEnvelope::Success { json }),
            Err(_) => parser_failure(Failure::new(FailureKind::OutputSerialization {
                target: parser_core::OutputTarget::ParseResult,
            })),
        },
        Err(error) => parser_failure(error),
    }
}

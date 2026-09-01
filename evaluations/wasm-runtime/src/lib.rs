use parser_core::{DiagnosticsMode, Failure, FailureKind, ParserError};
use parser_formats::{
    CsvOptions, InputSource, TextLimits, read_csv_bytes, read_input, read_txt_bytes,
    read_xlsx_bytes,
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(
    inline_js = "module.exports.fuzzy_parser_parse_entry = function() { if (globalThis.__fuzzyParserParseEntry) globalThis.__fuzzyParserParseEntry(); }"
)]
extern "C" {
    fn fuzzy_parser_parse_entry();
}

#[inline]
fn signal_parse_entry() {
    #[cfg(target_arch = "wasm32")]
    fuzzy_parser_parse_entry();
}

#[derive(Deserialize)]
pub struct NativeRequest {
    pub format: String,
    pub bytes: Vec<u8>,
    pub schema: String,
    pub filename: Option<String>,
}

#[derive(Serialize)]
pub struct NativeResponse {
    pub json: String,
}

fn failure(error: Failure) -> String {
    serde_json::to_string_pretty(&error.report(DiagnosticsMode::Safe))
        .expect("typed reports serialize")
}

fn parser_failure(error: ParserError) -> String {
    failure(Failure::from(&error))
}

/// The evaluation boundary: borrowed bytes plus declared format, optional filename and schema.
/// The string is the existing success or safe-error JSON, never an adapter envelope.
pub fn parse_bytes(
    format: &str,
    bytes: &[u8],
    filename: Option<&str>,
    schema_json: &str,
) -> String {
    let schema = match parser_schema::decode_execution_schema(schema_json) {
        Ok(schema) => schema,
        Err(error) => return failure(error),
    };
    let document = match format {
        "text" => match String::from_utf8(bytes.to_vec()) {
            Ok(text) => read_input(InputSource::Text(&text), TextLimits::default()),
            Err(error) => Err(ParserError::InvalidUtf8 {
                path: "<bytes>".to_owned(),
                valid_up_to: error.utf8_error().valid_up_to(),
            }),
        },
        "txt" => read_txt_bytes(filename, bytes, "<bytes>"),
        "csv" => read_csv_bytes(filename, bytes, "<bytes>", CsvOptions::default()),
        "xlsx" => read_xlsx_bytes(filename, bytes),
        _ => Err(ParserError::UnsupportedInput {
            source_type: format.to_owned(),
        }),
    };
    let document = match document {
        Ok(document) => document,
        Err(error) => return parser_failure(error),
    };
    let plan = match parser_schema::compile_schema(&schema) {
        Ok(plan) => plan,
        Err(error) => return failure(error),
    };
    signal_parse_entry();
    match serde_json::to_string_pretty(&parser_core::parse_document_with_plan(&document, &plan)) {
        Ok(json) => json,
        Err(_) => failure(Failure::new(FailureKind::OutputSerialization {
            target: parser_core::OutputTarget::ParseResult,
        })),
    }
}

#[wasm_bindgen]
pub fn parse_bytes_json(
    format: String,
    bytes: Vec<u8>,
    filename: Option<String>,
    schema: String,
) -> String {
    parse_bytes(&format, &bytes, filename.as_deref(), &schema)
}

pub fn native_oracle(request: NativeRequest) -> NativeResponse {
    NativeResponse {
        json: parse_bytes(
            &request.format,
            &request.bytes,
            request.filename.as_deref(),
            &request.schema,
        ),
    }
}

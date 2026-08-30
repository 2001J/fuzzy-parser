use parser_core::{DiagnosticsMode, Failure, FailureKind, OutputTarget};
use parser_formats::{
    CsvOptions, ExtractedTable, FileFormat, InputSource, TextLimits, TxtOptions,
    parse_extracted_table_with_plan, read_csv_table_with_options, read_csv_with_options,
    read_input, read_txt_with_options, read_xlsx, read_xlsx_table,
};
use std::{
    env, fs,
    io::{self, Read},
    path::PathBuf,
    process,
};

mod arguments;

fn main() {
    process::exit(run());
}

fn run() -> i32 {
    let invocation = match arguments::parse(env::args_os().skip(1).collect()) {
        Ok(invocation) => invocation,
        Err(()) => {
            eprintln!("usage: parser-cli --help");
            return 2;
        }
    };
    let diagnostics = invocation.diagnostics;
    match invocation.command {
        arguments::Command::Help => {
            println!("{}", arguments::HELP);
            0
        }
        arguments::Command::InspectPath { path, options } => {
            inspect_result(read_document(&path, options), diagnostics)
        }
        arguments::Command::InspectStdin => inspect_stdin(diagnostics),
        arguments::Command::InspectText(content) => match content.into_string() {
            Ok(content) => inspect_text(&content, diagnostics),
            Err(_) => {
                eprintln!("text argument must be valid UTF-8");
                2
            }
        },
        arguments::Command::ParsePath {
            path,
            schema,
            options,
        } => parse_path(path, schema, options, diagnostics),
        arguments::Command::ParseStdin { schema } => parse_stdin(schema, diagnostics),
        arguments::Command::SchemaPath { path, pretty } => {
            validate_schema_path(path, pretty, diagnostics)
        }
        arguments::Command::SchemaStdin => validate_schema_stdin(diagnostics),
        arguments::Command::SchemaText(content) => match content.into_string() {
            Ok(content) => validate_schema_input(&content, true, diagnostics, None),
            Err(_) => report_failure(Failure::new(FailureKind::SchemaInput), diagnostics),
        },
    }
}

fn read_document(
    path: &std::path::Path,
    options: TxtOptions,
) -> Result<parser_core::RawDocument, parser_core::ParserError> {
    match arguments::file_format(path) {
        Some(FileFormat::Txt) => read_txt_with_options(path, options),
        Some(FileFormat::Csv) => read_csv_with_options(path, CsvOptions::default()),
        Some(FileFormat::Xlsx) => read_xlsx(path),
        None => Err(parser_core::ParserError::UnsupportedInput {
            source_type: path
                .extension()
                .map(|value| value.to_string_lossy().into_owned())
                .unwrap_or_default(),
        }),
    }
}

fn inspect_stdin(diagnostics: DiagnosticsMode) -> i32 {
    let stdin = io::stdin();
    let mut reader = stdin.lock();
    inspect_result(
        read_input(InputSource::Stdin(&mut reader), TextLimits::default()),
        diagnostics,
    )
}

fn inspect_text(content: &str, diagnostics: DiagnosticsMode) -> i32 {
    inspect_result(
        read_input(InputSource::Text(content), TextLimits::default()),
        diagnostics,
    )
}

fn schema_failure(error: &parser_schema::SchemaParseError, path: Option<&PathBuf>) -> Failure {
    let failure = Failure::from(error);
    match path {
        Some(path) => failure.with_path(&path.to_string_lossy()),
        None => failure,
    }
}

fn schema_io_failure(error: &io::Error, path: Option<&PathBuf>) -> Failure {
    let failure = Failure::new(FailureKind::SchemaIo {
        kind: error.kind().into(),
    });
    match path {
        Some(path) => failure.with_path(&path.to_string_lossy()),
        None => failure,
    }
}

fn load_schema_file(path: PathBuf) -> Result<parser_schema::TargetSchema, Failure> {
    let input =
        fs::read_to_string(&path).map_err(|error| schema_io_failure(&error, Some(&path)))?;
    parser_schema::decode_execution_schema(&input)
        .map_err(|error| error.with_path(&path.to_string_lossy()))
}

fn parse_path(
    input_path: PathBuf,
    schema_path: PathBuf,
    options: arguments::ParsePathOptions,
    diagnostics: DiagnosticsMode,
) -> i32 {
    let schema = match load_schema_file(schema_path) {
        Ok(schema) => schema,
        Err(error) => return report_failure(error, diagnostics),
    };
    if let Some(table_options) = options.table {
        let table = match read_table(&input_path) {
            Ok(table) => table,
            Err(error) => return report_failure(Failure::from(&error), diagnostics),
        };
        let plan = match parser_schema::compile_schema(&schema) {
            Ok(plan) => plan,
            Err(error) => return report_failure(error, diagnostics),
        };
        match parse_extracted_table_with_plan(&table, &plan, &table_options) {
            Ok(response) => write_response(&response, diagnostics),
            Err(error) => report_failure(error, diagnostics),
        }
    } else {
        parse_with_schema(read_document(&input_path, options.txt), schema, diagnostics)
    }
}

fn read_table(path: &std::path::Path) -> Result<ExtractedTable, parser_core::ParserError> {
    match arguments::file_format(path) {
        Some(FileFormat::Csv) => read_csv_table_with_options(path, CsvOptions::default()),
        Some(FileFormat::Xlsx) => read_xlsx_table(path),
        _ => Err(parser_core::ParserError::UnsupportedInput {
            source_type: path
                .extension()
                .map(|value| value.to_string_lossy().into_owned())
                .unwrap_or_default(),
        }),
    }
}

fn parse_stdin(schema_path: PathBuf, diagnostics: DiagnosticsMode) -> i32 {
    let schema = match load_schema_file(schema_path) {
        Ok(schema) => schema,
        Err(error) => return report_failure(error, diagnostics),
    };
    let stdin = io::stdin();
    let mut reader = stdin.lock();
    parse_with_schema(
        read_input(InputSource::Stdin(&mut reader), TextLimits::default()),
        schema,
        diagnostics,
    )
}

fn parse_with_schema(
    document: Result<parser_core::RawDocument, parser_core::ParserError>,
    schema: parser_schema::TargetSchema,
    diagnostics: DiagnosticsMode,
) -> i32 {
    let document = match document {
        Ok(document) => document,
        Err(error) => return report_failure(Failure::from(&error), diagnostics),
    };
    let spec = match parser_schema::compile_schema(&schema) {
        Ok(spec) => spec,
        Err(error) => return report_failure(error, diagnostics),
    };
    let response = parser_core::parse_document_with_plan(&document, &spec);
    write_response(&response, diagnostics)
}

fn write_response(response: &parser_core::ParseResponse, diagnostics: DiagnosticsMode) -> i32 {
    match serde_json::to_string_pretty(response) {
        Ok(json) => {
            println!("{json}");
            0
        }
        Err(_) => report_failure(
            Failure::new(FailureKind::OutputSerialization {
                target: OutputTarget::ParseResult,
            }),
            diagnostics,
        ),
    }
}

fn validate_schema_path(path: PathBuf, pretty: bool, diagnostics: DiagnosticsMode) -> i32 {
    match fs::read_to_string(&path) {
        Ok(input) => validate_schema_input(&input, pretty, diagnostics, Some(&path)),
        Err(error) => report_failure(schema_io_failure(&error, Some(&path)), diagnostics),
    }
}

fn validate_schema_stdin(diagnostics: DiagnosticsMode) -> i32 {
    let mut input = String::new();
    match io::stdin().read_to_string(&mut input) {
        Ok(_) => validate_schema_input(&input, true, diagnostics, None),
        Err(error) => report_failure(schema_io_failure(&error, None), diagnostics),
    }
}

fn validate_schema_input(
    input: &str,
    pretty: bool,
    diagnostics: DiagnosticsMode,
    path: Option<&PathBuf>,
) -> i32 {
    match parser_schema::TargetSchema::from_json(input) {
        Ok(schema) => match schema.to_json() {
            Ok(json) => {
                if pretty {
                    println!("{json}");
                } else {
                    println!(
                        "{}",
                        serde_json::to_string(&schema).expect("validated schema should serialize")
                    );
                }
                0
            }
            Err(error) => {
                let mut failure = error.serialization_failure();
                if let Some(path) = path {
                    failure = failure.with_path(&path.to_string_lossy());
                }
                report_failure(failure, diagnostics)
            }
        },
        Err(error) => report_failure(schema_failure(&error, path), diagnostics),
    }
}

fn report_failure(error: Failure, diagnostics: DiagnosticsMode) -> i32 {
    eprintln!(
        "{}",
        serde_json::to_string_pretty(&error.report(diagnostics))
            .expect("typed error reports contain only serializable values")
    );
    1
}

fn inspect_result(
    result: Result<parser_core::RawDocument, parser_core::ParserError>,
    diagnostics: DiagnosticsMode,
) -> i32 {
    match result {
        Ok(document) => match serde_json::to_string_pretty(&document) {
            Ok(json) => {
                println!("{json}");
                0
            }
            Err(_) => report_failure(
                Failure::new(FailureKind::OutputSerialization {
                    target: OutputTarget::RawDocument,
                }),
                diagnostics,
            ),
        },
        Err(error) => report_failure(Failure::from(&error), diagnostics),
    }
}

#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;

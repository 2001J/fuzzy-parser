use parser_core::{DiagnosticsMode, Failure, FailureKind, OutputTarget};
use parser_formats::{
    CsvOptions, InputSource, TextLimits, read_csv_with_options, read_input, read_xlsx,
};
use std::{
    env, fs,
    io::{self, Read},
    path::PathBuf,
    process,
};

fn main() {
    process::exit(run());
}

fn run() -> i32 {
    let mut arguments = env::args_os();
    let _program = arguments.next();
    let first = arguments.next();
    let (diagnostics, command) = if first.as_deref() == Some(std::ffi::OsStr::new("--diagnostics"))
    {
        (DiagnosticsMode::Detailed, arguments.next())
    } else {
        (DiagnosticsMode::Safe, first)
    };

    match (
        command,
        arguments.next(),
        arguments.next(),
        arguments.next(),
    ) {
        (Some(flag), None, None, None) if flag == "--help" || flag == "-h" => {
            print_help();
            0
        }
        (Some(command), Some(action), Some(flag), None)
            if command == "schema"
                && action == "validate"
                && (flag == "--help" || flag == "-h") =>
        {
            print_help();
            0
        }
        (Some(command), Some(flag), None, None) if command == "inspect" && flag == "--stdin" => {
            inspect_stdin(diagnostics)
        }
        (Some(command), Some(flag), Some(content), None)
            if command == "inspect" && flag == "--text" =>
        {
            match content.into_string() {
                Ok(content) => inspect_text(&content, diagnostics),
                Err(_) => {
                    eprintln!("text argument must be valid UTF-8");
                    2
                }
            }
        }
        (Some(command), Some(path), None, None) if command == "inspect" => {
            inspect_path(PathBuf::from(path), diagnostics)
        }
        (Some(command), Some(action), Some(flag), None)
            if command == "schema" && action == "validate" && flag == "--stdin" =>
        {
            validate_schema_stdin(diagnostics)
        }
        (Some(command), Some(action), Some(path), None)
            if command == "schema" && action == "validate" =>
        {
            validate_schema_path(PathBuf::from(path), true, diagnostics)
        }
        (Some(command), Some(action), Some(flag), Some(content))
            if command == "schema" && action == "validate" && flag == "--text" =>
        {
            match content.into_string() {
                Ok(content) => validate_schema_input(&content, true, diagnostics, None),
                Err(_) => report_failure(Failure::new(FailureKind::SchemaInput), diagnostics),
            }
        }
        (Some(command), Some(action), Some(flag), Some(path))
            if command == "schema" && action == "validate" && flag == "--compact" =>
        {
            validate_schema_path(PathBuf::from(path), false, diagnostics)
        }
        (Some(command), Some(stdin_flag), Some(flag), Some(schema_path))
            if command == "parse" && stdin_flag == "--stdin" && flag == "--schema" =>
        {
            parse_stdin(PathBuf::from(schema_path), diagnostics)
        }
        (Some(command), Some(path), Some(flag), Some(schema_path))
            if command == "parse" && flag == "--schema" =>
        {
            parse_path(PathBuf::from(path), PathBuf::from(schema_path), diagnostics)
        }
        (Some(command), Some(flag), None, None)
            if command == "parse" && (flag == "--help" || flag == "-h") =>
        {
            print_help();
            0
        }
        _ => {
            eprintln!(
                "usage: parser-cli inspect <path> | --stdin | --text <content> | schema validate <path> | parse <path> --schema <path>"
            );
            2
        }
    }
}

fn print_help() {
    println!(
        "usage: parser-cli inspect <path> | --stdin | --text <content> | schema validate <path> | schema validate --stdin | schema validate --text <content> | parse <path> --schema <schema-path> | parse --stdin --schema <schema-path>"
    );
}

fn read_document(path: &PathBuf) -> Result<parser_core::RawDocument, parser_core::ParserError> {
    let extension = path.extension().and_then(|extension| extension.to_str());
    let is_csv = extension
        .map(|extension| extension.eq_ignore_ascii_case("csv"))
        .unwrap_or(false);
    let is_xlsx = extension
        .map(|extension| extension.eq_ignore_ascii_case("xlsx"))
        .unwrap_or(false);

    if is_csv {
        read_csv_with_options(path, CsvOptions::default())
    } else if is_xlsx {
        read_xlsx(path)
    } else {
        read_input(InputSource::TxtFile(path), TextLimits::default())
    }
}

fn inspect_path(path: PathBuf, diagnostics: DiagnosticsMode) -> i32 {
    inspect_result(read_document(&path), diagnostics)
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

fn parse_path(input_path: PathBuf, schema_path: PathBuf, diagnostics: DiagnosticsMode) -> i32 {
    let schema = match load_schema_file(schema_path) {
        Ok(schema) => schema,
        Err(error) => return report_failure(error, diagnostics),
    };
    parse_with_schema(read_document(&input_path), schema, diagnostics)
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
    match serde_json::to_string_pretty(&response) {
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

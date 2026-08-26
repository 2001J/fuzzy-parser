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

    match (
        arguments.next(),
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
            inspect_stdin()
        }
        (Some(command), Some(flag), Some(content), None)
            if command == "inspect" && flag == "--text" =>
        {
            match content.into_string() {
                Ok(content) => inspect_text(&content),
                Err(_) => {
                    eprintln!("text argument must be valid UTF-8");
                    2
                }
            }
        }
        (Some(command), Some(path), None, None) if command == "inspect" => {
            inspect_path(PathBuf::from(path))
        }
        (Some(command), Some(action), Some(flag), None)
            if command == "schema" && action == "validate" && flag == "--stdin" =>
        {
            validate_schema_stdin()
        }
        (Some(command), Some(action), Some(path), None)
            if command == "schema" && action == "validate" =>
        {
            validate_schema_path(PathBuf::from(path))
        }
        (Some(command), Some(action), Some(flag), Some(content))
            if command == "schema" && action == "validate" && flag == "--text" =>
        {
            match content.into_string() {
                Ok(content) => validate_schema_input(&content),
                Err(_) => schema_error(
                    "schema_input_error",
                    "schema text must be valid UTF-8".to_owned(),
                ),
            }
        }
        (Some(command), Some(action), Some(flag), Some(path))
            if command == "schema" && action == "validate" && flag == "--compact" =>
        {
            validate_schema_path_compact(PathBuf::from(path))
        }
        (Some(command), Some(stdin_flag), Some(flag), Some(schema_path))
            if command == "parse" && stdin_flag == "--stdin" && flag == "--schema" =>
        {
            parse_stdin(PathBuf::from(schema_path))
        }
        (Some(command), Some(path), Some(flag), Some(schema_path))
            if command == "parse" && flag == "--schema" =>
        {
            parse_path(PathBuf::from(path), PathBuf::from(schema_path))
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

fn inspect_path(path: PathBuf) -> i32 {
    inspect_result(read_document(&path))
}

fn inspect_stdin() -> i32 {
    let stdin = io::stdin();
    let mut reader = stdin.lock();
    inspect_result(read_input(
        InputSource::Stdin(&mut reader),
        TextLimits::default(),
    ))
}

fn inspect_text(content: &str) -> i32 {
    inspect_result(read_input(
        InputSource::Text(content),
        TextLimits::default(),
    ))
}

struct AssignmentSpec {
    fields: Vec<parser_core::AssignmentField>,
    enum_definitions: Vec<(String, Vec<String>)>,
}

fn assignment_spec(schema: &parser_schema::TargetSchema) -> Result<AssignmentSpec, String> {
    use parser_core::{AssignmentConstraint, AssignmentField, CandidateType};
    use parser_schema::{FieldConstraint, FieldType};

    let mut fields = Vec::new();
    let mut enum_definitions: Vec<(String, Vec<String>)> = Vec::new();

    for field in &schema.fields {
        let candidate_type = match &field.field_type {
            FieldType::Email => CandidateType::Email,
            FieldType::Integer => CandidateType::Integer,
            FieldType::Decimal => CandidateType::Decimal,
            FieldType::PhoneNumber => CandidateType::PhoneNumber,
            FieldType::Boolean => CandidateType::Boolean,
            FieldType::Date => CandidateType::Date,
            FieldType::Currency => CandidateType::Currency,
            FieldType::Enum { values } => {
                for value in values {
                    enum_definitions.push((value.value.clone(), value.aliases.clone()));
                }
                CandidateType::Enum
            }
            FieldType::Datetime => {
                return Err(format!(
                    "field \"{}\": field type \"datetime\" is not supported by the parser yet",
                    field.name
                ));
            }
            FieldType::Text => {
                return Err(format!(
                    "field \"{}\": field type \"text\" is not supported by the parser yet",
                    field.name
                ));
            }
            FieldType::PersonName => {
                return Err(format!(
                    "field \"{}\": field type \"person_name\" is not supported by the parser yet",
                    field.name
                ));
            }
        };

        let constraints = field
            .constraints
            .iter()
            .map(|constraint| match constraint {
                FieldConstraint::MinimumInteger(value) => {
                    AssignmentConstraint::MinimumInteger(*value)
                }
                FieldConstraint::MaximumInteger(value) => {
                    AssignmentConstraint::MaximumInteger(*value)
                }
                FieldConstraint::MinimumLength(value) => {
                    AssignmentConstraint::MinimumLength(*value)
                }
                FieldConstraint::MaximumLength(value) => {
                    AssignmentConstraint::MaximumLength(*value)
                }
            })
            .collect();

        fields.push(AssignmentField {
            name: field.name.clone(),
            aliases: field.aliases.clone(),
            candidate_type,
            required: field.required,
            multiple: field.multiple,
            unique: false,
            constraints,
            expected_column: None,
        });
    }

    Ok(AssignmentSpec {
        fields,
        enum_definitions,
    })
}

fn load_schema_file(path: PathBuf) -> Result<parser_schema::TargetSchema, (String, String)> {
    match fs::read_to_string(path) {
        Ok(input) => match parser_schema::TargetSchema::from_json(&input) {
            Ok(schema) => Ok(schema),
            Err(error) => {
                let code = match error {
                    parser_schema::SchemaParseError::InvalidJson(_) => "schema_parse_error",
                    parser_schema::SchemaParseError::InvalidSchema(_) => "schema_validation_error",
                };
                Err((code.to_owned(), error.to_string()))
            }
        },
        Err(error) => Err((
            "schema_io_error".to_owned(),
            format!("{}: {:?}", error, error.kind()),
        )),
    }
}

fn parse_path(input_path: PathBuf, schema_path: PathBuf) -> i32 {
    let schema = match load_schema_file(schema_path) {
        Ok(schema) => schema,
        Err((code, message)) => return schema_error(&code, message),
    };
    parse_with_schema(read_document(&input_path), schema)
}

fn parse_stdin(schema_path: PathBuf) -> i32 {
    let schema = match load_schema_file(schema_path) {
        Ok(schema) => schema,
        Err((code, message)) => return schema_error(&code, message),
    };
    let stdin = io::stdin();
    let mut reader = stdin.lock();
    parse_with_schema(
        read_input(InputSource::Stdin(&mut reader), TextLimits::default()),
        schema,
    )
}

fn parse_with_schema(
    document: Result<parser_core::RawDocument, parser_core::ParserError>,
    schema: parser_schema::TargetSchema,
) -> i32 {
    let document = match document {
        Ok(document) => document,
        Err(error) => {
            let output = serde_json::json!({
                "error": error,
                "message": error.to_string(),
            });
            eprintln!(
                "{}",
                serde_json::to_string_pretty(&output)
                    .expect("the structured parser error should be serializable")
            );
            return 1;
        }
    };

    let spec = match assignment_spec(&schema) {
        Ok(spec) => spec,
        Err(message) => return schema_error("schema_field_type_unsupported", message),
    };

    let response = parser_core::parse_document_with_assignment(
        &document,
        &spec.fields,
        &spec.enum_definitions,
        schema.record_name,
    );

    match serde_json::to_string_pretty(&response) {
        Ok(json) => {
            println!("{json}");
            0
        }
        Err(error) => {
            eprintln!("failed to serialize parse result: {error}");
            1
        }
    }
}

fn validate_schema_path(path: PathBuf) -> i32 {
    match fs::read_to_string(path) {
        Ok(input) => validate_schema_input(&input),
        Err(error) => schema_error("schema_io_error", format!("{}: {:?}", error, error.kind())),
    }
}

fn validate_schema_path_compact(path: PathBuf) -> i32 {
    match fs::read_to_string(path) {
        Ok(input) => validate_schema_input_with_format(&input, false),
        Err(error) => schema_error("schema_io_error", format!("{}: {:?}", error, error.kind())),
    }
}

fn validate_schema_stdin() -> i32 {
    let mut input = String::new();
    match io::stdin().read_to_string(&mut input) {
        Ok(_) => validate_schema_input(&input),
        Err(error) => schema_error("schema_io_error", format!("{}: {:?}", error, error.kind())),
    }
}

fn validate_schema_input(input: &str) -> i32 {
    validate_schema_input_with_format(input, true)
}

fn validate_schema_input_with_format(input: &str, pretty: bool) -> i32 {
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
            Err(error) => schema_error("schema_serialization_error", error.to_string()),
        },
        Err(error) => {
            let code = match error {
                parser_schema::SchemaParseError::InvalidJson(_) => "schema_parse_error",
                parser_schema::SchemaParseError::InvalidSchema(_) => "schema_validation_error",
            };
            schema_error(code, error.to_string())
        }
    }
}

fn schema_error(code: &str, message: String) -> i32 {
    let output = serde_json::json!({
        "error": {
            "code": code,
            "message": message,
        },
        "message": message,
    });
    eprintln!(
        "{}",
        serde_json::to_string_pretty(&output).expect("schema error should be serializable")
    );
    1
}

fn inspect_result(result: Result<parser_core::RawDocument, parser_core::ParserError>) -> i32 {
    match result {
        Ok(document) => match serde_json::to_string_pretty(&document) {
            Ok(json) => {
                println!("{json}");
                0
            }
            Err(error) => {
                eprintln!("failed to serialize inspection result: {error}");
                1
            }
        },
        Err(error) => {
            let output = serde_json::json!({
                "error": error,
                "message": error.to_string(),
            });
            eprintln!(
                "{}",
                serde_json::to_string_pretty(&output)
                    .expect("the structured parser error should be serializable")
            );
            1
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn empty_cli_test() {
        assert_eq!(2 + 2, 4);
    }
}

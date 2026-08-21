use parser_formats::{
    CsvOptions, InputSource, TextLimits, read_csv_with_options, read_input, read_xlsx,
};
use std::{env, fs, io, path::PathBuf, process};

fn main() {
    process::exit(run());
}

fn run() -> i32 {
    let mut arguments = env::args_os();
    let _program = arguments.next();

    match (arguments.next(), arguments.next(), arguments.next()) {
        (Some(command), Some(flag), None) if command == "inspect" && flag == "--stdin" => {
            inspect_stdin()
        }
        (Some(command), Some(flag), Some(content)) if command == "inspect" && flag == "--text" => {
            match content.into_string() {
                Ok(content) => inspect_text(&content),
                Err(_) => {
                    eprintln!("text argument must be valid UTF-8");
                    2
                }
            }
        }
        (Some(command), Some(path), None) if command == "inspect" => {
            inspect_path(PathBuf::from(path))
        }
        (Some(command), Some(action), Some(path))
            if command == "schema" && action == "validate" =>
        {
            validate_schema_path(PathBuf::from(path))
        }
        _ => {
            eprintln!(
                "usage: parser-cli inspect <path> | --stdin | --text <content> | schema validate <path>"
            );
            2
        }
    }
}

fn inspect_path(path: PathBuf) -> i32 {
    let extension = path.extension().and_then(|extension| extension.to_str());
    let is_csv = extension
        .map(|extension| extension.eq_ignore_ascii_case("csv"))
        .unwrap_or(false);
    let is_xlsx = extension
        .map(|extension| extension.eq_ignore_ascii_case("xlsx"))
        .unwrap_or(false);

    if is_csv {
        inspect_result(read_csv_with_options(&path, CsvOptions::default()))
    } else if is_xlsx {
        inspect_result(read_xlsx(&path))
    } else {
        inspect_result(read_input(
            InputSource::TxtFile(&path),
            TextLimits::default(),
        ))
    }
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

fn validate_schema_path(path: PathBuf) -> i32 {
    match fs::read_to_string(path) {
        Ok(input) => match parser_schema::TargetSchema::from_json(&input) {
            Ok(schema) => match schema.to_json() {
                Ok(json) => {
                    println!("{json}");
                    0
                }
                Err(error) => {
                    eprintln!("failed to serialize schema: {error}");
                    1
                }
            },
            Err(error) => {
                eprintln!("schema validation failed: {error}");
                1
            }
        },
        Err(error) => {
            eprintln!("failed to read schema: {error}");
            1
        }
    }
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

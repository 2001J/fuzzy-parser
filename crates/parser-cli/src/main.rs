use parser_formats::read_txt;
use std::{env, path::PathBuf, process};

fn main() {
    process::exit(run());
}

fn run() -> i32 {
    let mut arguments = env::args_os();
    let _program = arguments.next();

    match (arguments.next(), arguments.next(), arguments.next()) {
        (Some(command), Some(path), None) if command == "inspect" => inspect(PathBuf::from(path)),
        _ => {
            eprintln!("usage: parser-cli inspect <path>");
            2
        }
    }
}

fn inspect(path: PathBuf) -> i32 {
    match read_txt(&path) {
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

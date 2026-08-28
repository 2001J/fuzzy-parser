use parser_core::DiagnosticsMode;
use parser_formats::{EmptyFilePolicy, FileFormat, TxtOptions};
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

pub struct Invocation {
    pub diagnostics: DiagnosticsMode,
    pub command: Command,
}

pub enum Command {
    Help,
    InspectPath {
        path: PathBuf,
        options: TxtOptions,
    },
    InspectStdin,
    InspectText(OsString),
    ParsePath {
        path: PathBuf,
        schema: PathBuf,
        options: TxtOptions,
    },
    ParseStdin {
        schema: PathBuf,
    },
    SchemaPath {
        path: PathBuf,
        pretty: bool,
    },
    SchemaStdin,
    SchemaText(OsString),
}

/// Parse the entire invocation before any I/O. Values are never scanned for flags.
pub fn parse(arguments: Vec<OsString>) -> Result<Invocation, ()> {
    let (diagnostics, args) = match arguments.split_first() {
        Some((flag, rest)) if flag == "--diagnostics" => (DiagnosticsMode::Detailed, rest),
        _ => (DiagnosticsMode::Safe, arguments.as_slice()),
    };
    let command = match args {
        [flag] if is_help(flag) => Command::Help,
        [command, flag]
            if matches!(command.to_str(), Some("inspect" | "parse" | "schema"))
                && is_help(flag) =>
        {
            Command::Help
        }
        [command, action, flag] if command == "schema" && action == "validate" && is_help(flag) => {
            Command::Help
        }
        [command, flag] if command == "inspect" && flag == "--stdin" => Command::InspectStdin,
        [command, flag, content] if command == "inspect" && flag == "--text" => {
            Command::InspectText(content.clone())
        }
        [command, path, tail @ ..] if command == "inspect" => {
            let (path, options) = file_options(path, tail)?;
            Command::InspectPath { path, options }
        }
        [command, input, flag, schema]
            if command == "parse" && input == "--stdin" && flag == "--schema" =>
        {
            Command::ParseStdin {
                schema: path_argument(schema)?,
            }
        }
        [command, input, flag, schema, tail @ ..] if command == "parse" && flag == "--schema" => {
            let schema = path_argument(schema)?;
            let (path, options) = file_options(input, tail)?;
            Command::ParsePath {
                path,
                schema,
                options,
            }
        }
        [command, action, flag]
            if command == "schema" && action == "validate" && flag == "--stdin" =>
        {
            Command::SchemaStdin
        }
        [command, action, flag, content]
            if command == "schema" && action == "validate" && flag == "--text" =>
        {
            Command::SchemaText(content.clone())
        }
        [command, action, flag, path]
            if command == "schema" && action == "validate" && flag == "--compact" =>
        {
            Command::SchemaPath {
                path: path_argument(path)?,
                pretty: false,
            }
        }
        [command, action, path] if command == "schema" && action == "validate" => {
            Command::SchemaPath {
                path: path_argument(path)?,
                pretty: true,
            }
        }
        _ => return Err(()),
    };
    Ok(Invocation {
        diagnostics,
        command,
    })
}

fn is_help(value: &std::ffi::OsStr) -> bool {
    value == "--help" || value == "-h"
}

fn path_argument(value: &std::ffi::OsStr) -> Result<PathBuf, ()> {
    // ASCII '-' can be checked without lossy conversion of a native OS path.
    if value.as_encoded_bytes().starts_with(b"-") {
        Err(())
    } else {
        Ok(PathBuf::from(value))
    }
}

fn file_options(path: &std::ffi::OsStr, tail: &[OsString]) -> Result<(PathBuf, TxtOptions), ()> {
    let path = path_argument(path)?;
    let mut options = TxtOptions::default();
    let mut has_max_bytes = false;
    let mut has_empty = false;
    let mut pairs = tail.chunks_exact(2);
    for pair in &mut pairs {
        match pair[0].to_str() {
            Some("--max-bytes") if !has_max_bytes => {
                let value = pair[1].to_str().ok_or(())?;
                if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
                    return Err(());
                }
                options.limits.max_bytes = value.parse().map_err(|_| ())?;
                has_max_bytes = true;
            }
            Some("--empty") if !has_empty => {
                options.empty_policy = match pair[1].to_str() {
                    Some("accept") => EmptyFilePolicy::Accept,
                    Some("reject") => EmptyFilePolicy::Reject,
                    _ => return Err(()),
                };
                has_empty = true;
            }
            _ => return Err(()),
        }
    }
    if !pairs.remainder().is_empty()
        || (!tail.is_empty()
            && matches!(file_format(&path), Some(FileFormat::Csv | FileFormat::Xlsx)))
    {
        return Err(());
    }
    Ok((path, options))
}

/// Extension routing only. Adapters retain ownership of validation and reading.
pub fn file_format(path: &Path) -> Option<FileFormat> {
    match path.extension()?.to_str()? {
        extension if extension.eq_ignore_ascii_case("txt") => Some(FileFormat::Txt),
        extension if extension.eq_ignore_ascii_case("csv") => Some(FileFormat::Csv),
        extension if extension.eq_ignore_ascii_case("xlsx") => Some(FileFormat::Xlsx),
        _ => None,
    }
}

pub const HELP: &str = "usage: parser-cli [--diagnostics] <command>

  inspect <path> [TXT_OPTIONS]
  inspect --stdin
  inspect --text <content>
  parse <path> --schema <schema-path> [TXT_OPTIONS]
  parse --stdin --schema <schema-path>
  schema validate <path>
  schema validate --stdin
  schema validate --text <content>
  schema validate --compact <path>

Paths select TXT, CSV or XLSX by extension (case-insensitive); stdin/text are plain text.
TXT_OPTIONS (TXT file input only, trailing, either order, each at most once):
  --max-bytes N          ASCII decimal usize, zero allowed; default 1048576.
  --empty accept|reject Zero-byte policy; default accept. Whitespace is nonempty.
TXT line limit remains 65536 bytes. Overrides never affect schema files, stdin,
inline text, CSV or XLSX. CSV/XLSX readers do not have these byte limits.
Use ./-name or an absolute path for names beginning '-'; no -- terminator.
--diagnostics is allowed only once, before the command; it may expose private context.
-h/--help alone works at root, inspect, parse, schema and schema validate.
Extra, duplicate, unknown or misplaced arguments are rejected.
Data: JSON stdout, exit 0. Processing errors: JSON stderr, exit 1.
Usage errors: plain stderr, exit 2. Help: plain stdout, exit 0.";

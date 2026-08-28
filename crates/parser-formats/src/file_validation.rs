use crate::DEFAULT_MAX_TEXT_BYTES;
use parser_core::ParserError;
use std::{
    fs::{self, File, Metadata},
    path::Path,
};

/// Extension eligibility only, not content detection or an extraction guarantee.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileFormat {
    Txt,
    Csv,
    Xlsx,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum EmptyFilePolicy {
    /// Compatibility default: zero bytes are allowed by validation.
    #[default]
    Accept,
    Reject,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileValidationOptions {
    pub enabled_formats: Vec<FileFormat>,
    pub max_bytes: usize,
    pub empty_policy: EmptyFilePolicy,
}

impl Default for FileValidationOptions {
    fn default() -> Self {
        Self {
            enabled_formats: vec![FileFormat::Txt, FileFormat::Csv, FileFormat::Xlsx],
            max_bytes: DEFAULT_MAX_TEXT_BYTES,
            empty_policy: EmptyFilePolicy::Accept,
        }
    }
}

/// Checked metadata is a snapshot. The opened file may still change concurrently.
#[derive(Debug)]
pub struct ValidatedFile {
    file: File,
    pub format: FileFormat,
    pub size_bytes: u64,
}

impl ValidatedFile {
    /// Consume the already-open handle instead of looking up the path again.
    /// Callers must still bound reads and enforce their post-read empty policy.
    pub fn into_file(self) -> File {
        self.file
    }
}

/// Follow symlinks and check type, extension, size and readability before decoding.
///
/// Prechecks avoid opening obvious special files, but cannot eliminate a race
/// replacing the path before open. This is not a sandbox or immutable snapshot.
/// CSV/XLSX eligibility does not integrate or resource-bound those adapters.
pub fn open_validated_file(
    path: impl AsRef<Path>,
    options: &FileValidationOptions,
) -> Result<ValidatedFile, ParserError> {
    let path = path.as_ref();
    let source = path.to_string_lossy();
    let io_error = |error: std::io::Error| ParserError::Io {
        path: source.to_string(),
        kind: error.kind().into(),
    };
    let metadata = fs::metadata(path).map_err(io_error)?;
    check_regular(&metadata, &source)?;
    let format = enabled_format(path, options)?;
    check_size(&metadata, &source, options)?;
    let file = File::open(path).map_err(io_error)?;
    let metadata = file.metadata().map_err(io_error)?;
    check_regular(&metadata, &source)?;
    check_size(&metadata, &source, options)?;
    Ok(ValidatedFile {
        file,
        format,
        size_bytes: metadata.len(),
    })
}

fn enabled_format(path: &Path, options: &FileValidationOptions) -> Result<FileFormat, ParserError> {
    let extension = path.extension().and_then(|value| value.to_str());
    let format = match extension {
        Some(value) if value.eq_ignore_ascii_case("txt") => Some(FileFormat::Txt),
        Some(value) if value.eq_ignore_ascii_case("csv") => Some(FileFormat::Csv),
        Some(value) if value.eq_ignore_ascii_case("xlsx") => Some(FileFormat::Xlsx),
        _ => None,
    };
    format
        .filter(|format| options.enabled_formats.contains(format))
        .ok_or_else(|| ParserError::UnsupportedInput {
            source_type: path
                .extension()
                .map(|value| value.to_string_lossy().into_owned())
                .unwrap_or_default(),
        })
}

fn check_regular(metadata: &Metadata, source: &str) -> Result<(), ParserError> {
    if !metadata.is_file() {
        return Err(ParserError::NotRegularFile {
            path: source.to_owned(),
        });
    }
    Ok(())
}

fn check_size(
    metadata: &Metadata,
    source: &str,
    options: &FileValidationOptions,
) -> Result<(), ParserError> {
    if metadata.len() > options.max_bytes as u64 {
        return Err(ParserError::FileTooLarge {
            path: source.to_owned(),
            limit: options.max_bytes as u64,
            actual: metadata.len(),
        });
    }
    check_empty(metadata.len(), source, options.empty_policy)
}

#[cfg(test)]
#[path = "../tests/unit/file_validation.rs"]
mod tests;

pub(crate) fn check_empty(
    size: u64,
    source: &str,
    policy: EmptyFilePolicy,
) -> Result<(), ParserError> {
    if size == 0 && policy == EmptyFilePolicy::Reject {
        return Err(ParserError::EmptyInput {
            path: source.to_owned(),
        });
    }
    Ok(())
}

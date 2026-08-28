use super::*;

#[test]
fn empty_cli_test() {
    assert_eq!(2 + 2, 4);
}

#[test]
fn schema_io_conversion_retains_typed_kinds_and_only_explicit_path_context() {
    for kind in [
        io::ErrorKind::NotFound,
        io::ErrorKind::PermissionDenied,
        io::ErrorKind::InvalidInput,
        io::ErrorKind::InvalidData,
        io::ErrorKind::Other,
    ] {
        let error = io::Error::new(kind, "private upstream schema prose");
        let path = PathBuf::from("/private/schema 東京.json");
        let failure = schema_io_failure(&error, Some(&path));
        assert_eq!(failure.kind, FailureKind::SchemaIo { kind: kind.into() });
        assert!(
            !failure
                .report(DiagnosticsMode::Safe)
                .message()
                .contains("private")
        );
        let detailed = failure.report(DiagnosticsMode::Detailed);
        assert_eq!(
            detailed.error.diagnostics.as_ref().unwrap().path.as_deref(),
            path.to_str()
        );
        assert!(!detailed.message().contains("upstream"));
        assert!(
            schema_io_failure(&error, None)
                .payload(DiagnosticsMode::Detailed)
                .diagnostics
                .is_none()
        );
    }
}

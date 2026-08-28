use super::*;

#[test]
fn file_validation_errors_have_exact_safe_and_detailed_contracts() {
    let path = "/synthetic/private/東京\n\u{1b}.txt";
    for (cause, code, mut payload, message) in [
        (
            ParserError::NotRegularFile { path: path.into() },
            "not_regular_file",
            serde_json::json!({"code":"not_regular_file"}),
            "input is not a regular file".to_owned(),
        ),
        (
            ParserError::EmptyInput { path: path.into() },
            "empty_input",
            serde_json::json!({"code":"empty_input"}),
            "empty input is not allowed".to_owned(),
        ),
        (
            ParserError::FileTooLarge {
                path: path.into(),
                limit: 1048576,
                actual: u64::MAX,
            },
            "file_too_large",
            serde_json::json!({"code":"file_too_large","limit":1048576,"actual":u64::MAX}),
            "file exceeds the 1048576-byte limit (18446744073709551615 bytes)".to_owned(),
        ),
    ] {
        assert_eq!(cause.code(), code);
        payload["error_contract_version"] = "0.1".into();
        assert_eq!(serde_json::to_value(&cause).unwrap(), payload);
        assert_eq!(
            serde_json::to_value(Failure::from(&cause)).unwrap(),
            payload
        );
        assert_eq!(cause.to_string(), message);
        assert_eq!(Failure::from(&cause).to_string(), message);
        let safe = cause.report(DiagnosticsMode::Safe);
        assert_eq!(
            serde_json::to_value(&safe).unwrap(),
            serde_json::json!({"error":payload,"message":message})
        );
        assert_eq!(safe.to_string(), message);
        assert!(!serde_json::to_string(&safe).unwrap().contains("private"));
        let detailed = cause.report(DiagnosticsMode::Detailed);
        payload["diagnostics"] = serde_json::json!({"path":path});
        let expected_message = format!(
            "{message} [diagnostics: {}]",
            serde_json::to_string(&serde_json::json!({"path":path})).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&detailed).unwrap(),
            serde_json::json!({"error":payload,"message":expected_message})
        );
        assert_eq!(detailed.to_string(), expected_message);
        assert!(!detailed.to_string().contains('\n'));
        assert!(!detailed.to_string().contains('\u{1b}'));
        for report in [safe, detailed] {
            let encoded = serde_json::to_string(&report).unwrap();
            assert_eq!(
                serde_json::from_str::<ErrorReport>(&encoded).unwrap(),
                report
            );
            let encoded_payload = serde_json::to_string(&report.error).unwrap();
            assert_eq!(
                serde_json::from_str::<ErrorPayload>(&encoded_payload).unwrap(),
                report.error
            );
            assert!(serde_json::from_str::<ParserError>(&encoded_payload).is_err());
        }
    }
}

#[test]
fn new_file_causes_keep_private_fields_without_changing_legacy_decoding() {
    for (json, cause) in [
        (
            serde_json::json!({"code":"not_regular_file","path":"private"}),
            ParserError::NotRegularFile {
                path: "private".into(),
            },
        ),
        (
            serde_json::json!({"code":"empty_input","path":"private"}),
            ParserError::EmptyInput {
                path: "private".into(),
            },
        ),
        (
            serde_json::json!({"code":"file_too_large","path":"private","limit":0,"actual":u64::MAX}),
            ParserError::FileTooLarge {
                path: "private".into(),
                limit: 0,
                actual: u64::MAX,
            },
        ),
    ] {
        assert_eq!(serde_json::from_value::<ParserError>(json).unwrap(), cause);
    }
    // The original cause fixture and exact old safe payloads remain covered by
    // legacy_parser_error_json_retains_original_cause_data and its companion tests.
    for cause in [
        ParserError::NotRegularFile {
            path: String::new(),
        },
        ParserError::EmptyInput {
            path: String::new(),
        },
        ParserError::FileTooLarge {
            path: String::new(),
            limit: 0,
            actual: 1,
        },
    ] {
        assert!(
            cause
                .report(DiagnosticsMode::Detailed)
                .error
                .diagnostics
                .is_none()
        );
    }
}

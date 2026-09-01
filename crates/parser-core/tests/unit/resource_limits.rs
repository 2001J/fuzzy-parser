use super::*;

fn document(values: &[&str]) -> RawDocument {
    RawDocument::new(
        "limited",
        SourceMetadata {
            source_type: SourceType::Text,
            file_name: None,
            mime_type: None,
            size_bytes: None,
            delimiter: None,
        },
        values
            .iter()
            .enumerate()
            .map(|(index, value)| RawBlock {
                id: format!("block-{}", index + 1),
                value: RawValue::text(*value),
                location: SourceLocation::default(),
            })
            .collect(),
    )
}

fn plan() -> ParsePlan {
    ParsePlan::new(Vec::new(), None)
}

#[test]
fn resource_limit_safe_rendering_uses_fixed_wire_names_not_debug_variants() {
    let cases = [
        (ResourceLimitKind::CsvBytes, "csv_bytes"),
        (ResourceLimitKind::CsvRows, "csv_rows"),
        (ResourceLimitKind::CsvCells, "csv_cells"),
        (ResourceLimitKind::XlsxBytes, "xlsx_bytes"),
        (ResourceLimitKind::XlsxSheets, "xlsx_sheets"),
        (ResourceLimitKind::XlsxCells, "xlsx_cells"),
        (ResourceLimitKind::SchemaBytes, "schema_bytes"),
        (ResourceLimitKind::SchemaFields, "schema_fields"),
        (ResourceLimitKind::SchemaAliases, "schema_aliases"),
        (ResourceLimitKind::SchemaNesting, "schema_nesting"),
        (ResourceLimitKind::Records, "records"),
        (ResourceLimitKind::ResponseBytes, "response_bytes"),
    ];
    for (resource, name) in cases {
        let report = Failure::new(FailureKind::ResourceLimit {
            resource,
            limit: 7,
            actual: 8,
        })
        .report(DiagnosticsMode::Safe);
        assert_eq!(
            report.message(),
            format!("resource limit {name} exceeded: limit 7, actual 8")
        );
        assert_eq!(
            serde_json::to_value(&report).unwrap(),
            serde_json::json!({
                "error": {
                    "error_contract_version": "0.1",
                    "code": "resource_limit",
                    "resource": name,
                    "limit": 7,
                    "actual": 8
                },
                "message": format!("resource limit {name} exceeded: limit 7, actual 8")
            })
        );
        assert!(!report.message().contains(&format!("{resource:?}")));
    }
}

#[test]
fn parsed_record_limit_accepts_exact_and_rejects_one_over_actual_records() {
    let document = document(&["first", "second"]);
    let exact = ParseLimits {
        max_records: 2,
        max_response_bytes: usize::MAX,
    };
    assert!(parse_document_with_plan_with_limits(&document, &plan(), exact).is_ok());

    let failure = parse_document_with_plan_with_limits(
        &document,
        &plan(),
        ParseLimits {
            max_records: 1,
            ..exact
        },
    )
    .unwrap_err();
    assert_eq!(
        failure.kind,
        FailureKind::ResourceLimit {
            resource: ResourceLimitKind::Records,
            limit: 1,
            actual: 2,
        }
    );
}

#[test]
fn serialized_response_limit_accepts_exact_and_rejects_one_over_without_a_second_buffer() {
    let document = document(&["email: ada@example.test"]);
    let response = parse_document_with_plan(&document, &plan());
    let bytes = serde_json::to_vec(&response).unwrap().len();
    let exact = ParseLimits {
        max_records: usize::MAX,
        max_response_bytes: bytes,
    };
    enforce_parse_response_limits(&response, exact).unwrap();

    let failure = enforce_parse_response_limits(
        &response,
        ParseLimits {
            max_response_bytes: bytes - 1,
            ..exact
        },
    )
    .unwrap_err();
    assert_eq!(
        failure.kind,
        FailureKind::ResourceLimit {
            resource: ResourceLimitKind::ResponseBytes,
            limit: (bytes - 1) as u64,
            actual: bytes as u64,
        }
    );
}

use super::*;

fn block(sheet: &str, row: usize, column: usize, value: RawValue) -> RawBlock {
    RawBlock {
        id: format!("{sheet}-{row}-{column}"),
        value,
        location: SourceLocation {
            row: Some(row),
            column: Some(column),
            sheet: Some(sheet.to_owned()),
            ..SourceLocation::default()
        },
    }
}

fn inventory(document: &RawDocument) -> TableInventory {
    TableInventory {
        source_type: SourceType::Xlsx,
        sheets: vec![
            TableInventorySheet {
                original_index: 1,
                name: Some("Zulu".to_owned()),
                rows: (1..=3)
                    .map(|source_row| TableInventoryRow {
                        source_row,
                        block_indices: document
                            .blocks
                            .iter()
                            .enumerate()
                            .filter(|(_, block)| {
                                block.location.sheet.as_deref() == Some("Zulu")
                                    && block.location.row == Some(source_row)
                            })
                            .map(|(index, _)| index)
                            .collect(),
                        blank: false,
                        byte_span: None,
                        line_start: None,
                        line_end: None,
                    })
                    .collect(),
                unsupported_metadata: Vec::new(),
            },
            TableInventorySheet {
                original_index: 2,
                name: Some("Empty".to_owned()),
                rows: Vec::new(),
                unsupported_metadata: Vec::new(),
            },
            TableInventorySheet {
                original_index: 3,
                name: Some("Alpha".to_owned()),
                rows: (1..=2)
                    .map(|source_row| TableInventoryRow {
                        source_row,
                        block_indices: document
                            .blocks
                            .iter()
                            .enumerate()
                            .filter(|(_, block)| {
                                block.location.sheet.as_deref() == Some("Alpha")
                                    && block.location.row == Some(source_row)
                            })
                            .map(|(index, _)| index)
                            .collect(),
                        blank: false,
                        byte_span: None,
                        line_start: None,
                        line_end: None,
                    })
                    .collect(),
                unsupported_metadata: Vec::new(),
            },
        ],
    }
}

fn fixture() -> (RawDocument, ParsePlan) {
    let document = RawDocument::new(
        "selection",
        SourceMetadata {
            source_type: SourceType::Xlsx,
            file_name: Some("selection.xlsx".to_owned()),
            mime_type: None,
            size_bytes: None,
            delimiter: None,
        },
        vec![
            block("Zulu", 1, 1, RawValue::text("preamble")),
            block("Zulu", 2, 1, RawValue::text("Email")),
            block("Zulu", 2, 2, RawValue::text("Age")),
            block("Zulu", 3, 1, RawValue::text("ada@example.test")),
            block("Zulu", 3, 2, RawValue::Integer(30)),
            block("Alpha", 1, 1, RawValue::text("first@example.test")),
            block("Alpha", 2, 1, RawValue::text("second@example.test")),
        ],
    );
    let plan = ParsePlan::new(
        vec![PlanField::new(
            AssignmentField {
                name: "email".to_owned(),
                aliases: Vec::new(),
                candidate_type: CandidateType::Email,
                required: false,
                multiple: false,
                unique: false,
                constraints: Vec::new(),
                expected_column: None,
            },
            Vec::new(),
        )],
        None,
    );
    (document, plan)
}

#[test]
fn explicit_sheet_order_header_and_empty_sheet_are_preserved() {
    let (document, plan) = fixture();
    let inventory = inventory(&document);
    let options = TableSelectionOptions {
        header: HeaderSelection::SchemaSearch { max_rows: 3 },
        rows: RowSelection::default(),
        sheets: SheetSelection::Selected(vec![
            SheetSelector::Name("Zulu".to_owned()),
            SheetSelector::Index(2),
        ]),
    };
    let response =
        parse_document_with_plan_and_table_selection(&document, &plan, &inventory, &options)
            .unwrap();
    let ParseContent::Table { sheets } = &response.content else {
        panic!("opt-in table parse must return table content");
    };
    assert_eq!(sheets.len(), 2);
    assert_eq!(sheets[0].sheet.as_deref(), Some("Zulu"));
    assert_eq!(sheets[0].records[0].source_row, 3);
    assert_eq!(sheets[1].sheet.as_deref(), Some("Empty"));
    assert!(sheets[1].records.is_empty());
    let table = response.source_evidence.unwrap().table.unwrap();
    assert_eq!(table.sheets[0].selection_order, Some(1));
    assert_eq!(table.sheets[1].selection_order, Some(2));
    assert_eq!(table.sheets[2].selection_order, None);
    assert_eq!(table.sheets[0].rows[0].role, TableRowRole::Preamble);
    assert_eq!(table.sheets[0].rows[1].role, TableRowRole::Header);
}

#[test]
fn no_header_keeps_the_all_text_first_row_and_exclusion_wins() {
    let (document, plan) = fixture();
    let inventory = inventory(&document);
    let options = TableSelectionOptions {
        header: HeaderSelection::None,
        rows: RowSelection {
            include: vec![InclusiveRowRange::new(1, 3)],
            exclude: vec![InclusiveRowRange::new(2, 2)],
        },
        sheets: SheetSelection::Selected(vec![SheetSelector::Index(1)]),
    };
    let response =
        parse_document_with_plan_and_table_selection(&document, &plan, &inventory, &options)
            .unwrap();
    let ParseContent::Table { sheets } = response.content else {
        panic!("expected table content");
    };
    assert_eq!(
        sheets[0]
            .records
            .iter()
            .map(|record| record.source_row)
            .collect::<Vec<_>>(),
        vec![1, 3]
    );
}

#[test]
fn schema_search_requires_a_unique_positive_best_row() {
    let (document, plan) = fixture();
    let inventory = inventory(&document);
    let options = TableSelectionOptions {
        header: HeaderSelection::SchemaSearch { max_rows: 3 },
        rows: RowSelection::default(),
        sheets: SheetSelection::Selected(vec![SheetSelector::Index(1)]),
    };
    let response =
        parse_document_with_plan_and_table_selection(&document, &plan, &inventory, &options)
            .unwrap();
    let ParseContent::Table { sheets } = response.content else {
        panic!("expected table content");
    };
    assert_eq!(sheets[0].header.context().unwrap().source_row, 2);
    assert_eq!(sheets[0].records.len(), 1);
}

#[test]
fn schema_search_no_match_and_tie_keep_every_row_with_deterministic_warnings() {
    let (document, plan) = fixture();
    let inventory = inventory(&document);
    let no_match = parse_document_with_plan_and_table_selection(
        &document,
        &plan,
        &inventory,
        &TableSelectionOptions {
            header: HeaderSelection::SchemaSearch { max_rows: 2 },
            sheets: SheetSelection::Selected(vec![SheetSelector::Name("Alpha".to_owned())]),
            ..TableSelectionOptions::default()
        },
    )
    .unwrap();
    let ParseContent::Table { sheets } = &no_match.content else {
        panic!("expected table content");
    };
    assert!(matches!(
        &sheets[0].header,
        HeaderExtraction::NotDetected { code, .. } if code == "header_search_no_match"
    ));
    assert_eq!(sheets[0].records.len(), 2);
    assert_eq!(no_match.warnings[0].code, "header_search_no_match");

    let mut tied_document = document.clone();
    tied_document.blocks[0].value = RawValue::text("Email");
    let tied = parse_document_with_plan_and_table_selection(
        &tied_document,
        &plan,
        &inventory,
        &TableSelectionOptions {
            header: HeaderSelection::SchemaSearch { max_rows: 2 },
            sheets: SheetSelection::Selected(vec![SheetSelector::Index(1)]),
            ..TableSelectionOptions::default()
        },
    )
    .unwrap();
    let ParseContent::Table { sheets } = &tied.content else {
        panic!("expected table content");
    };
    assert!(matches!(
        &sheets[0].header,
        HeaderExtraction::NotDetected { code, .. } if code == "header_search_ambiguous"
    ));
    assert_eq!(sheets[0].records.len(), 3);
    assert_eq!(tied.warnings[0].code, "header_search_ambiguous");
}

#[test]
fn explicit_header_accepts_typed_blank_and_duplicate_cells_verbatim() {
    let document = table_document(vec![
        table_block(None, 1, 1, RawValue::Integer(42)),
        table_block(None, 1, 2, RawValue::Null),
        table_block(None, 1, 3, RawValue::text("Age")),
        table_block(None, 1, 4, RawValue::text("Age")),
        table_block(None, 2, 1, RawValue::text("ada@example.test")),
    ]);
    let inventory = TableInventory {
        source_type: SourceType::Csv,
        sheets: vec![TableInventorySheet {
            original_index: 1,
            name: None,
            rows: vec![
                TableInventoryRow {
                    source_row: 1,
                    block_indices: vec![0, 1, 2, 3],
                    blank: false,
                    byte_span: None,
                    line_start: None,
                    line_end: None,
                },
                TableInventoryRow {
                    source_row: 2,
                    block_indices: vec![4],
                    blank: false,
                    byte_span: None,
                    line_start: None,
                    line_end: None,
                },
            ],
            unsupported_metadata: Vec::new(),
        }],
    };
    let (_, plan) = fixture();
    let response = parse_document_with_plan_and_table_selection(
        &document,
        &plan,
        &inventory,
        &TableSelectionOptions {
            header: HeaderSelection::Row(1),
            ..TableSelectionOptions::default()
        },
    )
    .unwrap();
    let ParseContent::Table { sheets } = response.content else {
        panic!("expected table content");
    };
    assert_eq!(
        sheets[0].header.context().unwrap().labels,
        vec![
            (1, "42".to_owned()),
            (2, String::new()),
            (3, "Age".to_owned()),
            (4, "Age".to_owned()),
        ]
    );
    assert_eq!(sheets[0].records[0].source_row, 2);
}

#[test]
fn all_sheets_keep_legacy_lexicographic_output_order() {
    let (document, plan) = fixture();
    let response = parse_document_with_plan_and_table_selection(
        &document,
        &plan,
        &inventory(&document),
        &TableSelectionOptions::default(),
    )
    .unwrap();
    let ParseContent::Table { sheets } = response.content else {
        panic!("expected table content");
    };
    assert_eq!(
        sheets
            .iter()
            .map(|sheet| sheet.sheet.as_deref())
            .collect::<Vec<_>>(),
        vec![Some("Alpha"), Some("Empty"), Some("Zulu")]
    );
}

#[test]
fn every_table_selection_failure_reason_is_reachable() {
    let (document, plan) = fixture();
    let inventory = inventory(&document);
    let cases = vec![
        (
            TableSelectionOptions {
                sheets: SheetSelection::Selected(Vec::new()),
                ..TableSelectionOptions::default()
            },
            TableSelectionReason::EmptySheetSelection,
        ),
        (
            TableSelectionOptions {
                sheets: SheetSelection::Selected(vec![
                    SheetSelector::Name("Zulu".to_owned()),
                    SheetSelector::Index(1),
                ]),
                ..TableSelectionOptions::default()
            },
            TableSelectionReason::DuplicateSheetSelection,
        ),
        (
            TableSelectionOptions {
                sheets: SheetSelection::Selected(vec![SheetSelector::Name("missing".to_owned())]),
                ..TableSelectionOptions::default()
            },
            TableSelectionReason::MissingSheet,
        ),
        (
            TableSelectionOptions {
                sheets: SheetSelection::Selected(vec![SheetSelector::Index(99)]),
                ..TableSelectionOptions::default()
            },
            TableSelectionReason::SheetIndexOutOfRange,
        ),
        (
            TableSelectionOptions {
                rows: RowSelection {
                    include: vec![InclusiveRowRange::new(3, 2)],
                    exclude: Vec::new(),
                },
                ..TableSelectionOptions::default()
            },
            TableSelectionReason::InvalidRowRange,
        ),
        (
            TableSelectionOptions {
                rows: RowSelection {
                    include: vec![InclusiveRowRange::new(1, 2), InclusiveRowRange::new(2, 3)],
                    exclude: Vec::new(),
                },
                ..TableSelectionOptions::default()
            },
            TableSelectionReason::OverlappingRowRange,
        ),
        (
            TableSelectionOptions {
                rows: RowSelection {
                    include: vec![InclusiveRowRange::new(99, 99)],
                    exclude: Vec::new(),
                },
                sheets: SheetSelection::Selected(vec![SheetSelector::Index(1)]),
                ..TableSelectionOptions::default()
            },
            TableSelectionReason::RowNotFound,
        ),
        (
            TableSelectionOptions {
                header: HeaderSelection::Row(99),
                sheets: SheetSelection::Selected(vec![SheetSelector::Index(1)]),
                ..TableSelectionOptions::default()
            },
            TableSelectionReason::HeaderNotFound,
        ),
        (
            TableSelectionOptions {
                header: HeaderSelection::Row(2),
                rows: RowSelection {
                    include: Vec::new(),
                    exclude: vec![InclusiveRowRange::new(2, 2)],
                },
                sheets: SheetSelection::Selected(vec![SheetSelector::Index(1)]),
            },
            TableSelectionReason::HeaderConflict,
        ),
    ];
    for (options, reason) in cases {
        let error =
            parse_document_with_plan_and_table_selection(&document, &plan, &inventory, &options)
                .unwrap_err();
        assert_eq!(error.kind, FailureKind::TableSelection { reason });
    }

    let mut text_document = document.clone();
    text_document.source.source_type = SourceType::Text;
    let mut text_inventory = inventory.clone();
    text_inventory.source_type = SourceType::Text;
    assert_eq!(
        parse_document_with_plan_and_table_selection(
            &text_document,
            &plan,
            &text_inventory,
            &TableSelectionOptions::default(),
        )
        .unwrap_err()
        .kind,
        FailureKind::TableSelection {
            reason: TableSelectionReason::UnsupportedSource
        }
    );
}

#[test]
fn sheet_index_uses_original_index_metadata() {
    let (document, plan) = fixture();
    let mut inventory = inventory(&document);
    inventory.sheets[0].original_index = 10;
    let response = parse_document_with_plan_and_table_selection(
        &document,
        &plan,
        &inventory,
        &TableSelectionOptions {
            sheets: SheetSelection::Selected(vec![SheetSelector::Index(10)]),
            ..TableSelectionOptions::default()
        },
    )
    .unwrap();
    let ParseContent::Table { sheets } = response.content else {
        panic!("expected table content");
    };
    assert_eq!(sheets[0].sheet.as_deref(), Some("Zulu"));
}

#[test]
fn semantic_selection_errors_have_typed_safe_and_detailed_reports() {
    for (reason, message) in [
        (
            TableSelectionReason::UnsupportedSource,
            "table selection is not supported for this source",
        ),
        (
            TableSelectionReason::EmptySheetSelection,
            "sheet selection must not be empty",
        ),
        (
            TableSelectionReason::DuplicateSheetSelection,
            "the same sheet was selected more than once",
        ),
        (
            TableSelectionReason::MissingSheet,
            "selected sheet was not found",
        ),
        (
            TableSelectionReason::SheetIndexOutOfRange,
            "selected sheet index is out of range",
        ),
        (
            TableSelectionReason::InvalidRowRange,
            "row range is invalid",
        ),
        (
            TableSelectionReason::OverlappingRowRange,
            "row ranges overlap",
        ),
        (
            TableSelectionReason::RowNotFound,
            "selected row was not found",
        ),
        (
            TableSelectionReason::HeaderNotFound,
            "selected header row was not found",
        ),
        (
            TableSelectionReason::HeaderConflict,
            "header selection conflicts with row selection",
        ),
    ] {
        assert_eq!(
            table_selection_failure(reason, None)
                .report(DiagnosticsMode::Safe)
                .message(),
            message
        );
    }

    let failure = table_selection_failure(
        TableSelectionReason::MissingSheet,
        Some("private 東京\n\u{1b}"),
    );
    assert_eq!(
        serde_json::to_value(failure.report(DiagnosticsMode::Safe)).unwrap(),
        serde_json::json!({
            "error": {
                "error_contract_version": "0.1",
                "code": "table_selection_error",
                "reason": "missing_sheet"
            },
            "message": "selected sheet was not found"
        })
    );
    let detailed = serde_json::to_value(failure.report(DiagnosticsMode::Detailed)).unwrap();
    assert_eq!(
        detailed["error"]["diagnostics"]["sheet"],
        "private 東京\n\u{1b}"
    );
    assert!(
        !detailed["message"]
            .as_str()
            .unwrap()
            .contains(['\n', '\u{1b}'])
    );
    let payload = failure.payload(DiagnosticsMode::Safe);
    assert_eq!(
        serde_json::from_value::<ErrorPayload>(serde_json::to_value(&payload).unwrap()).unwrap(),
        payload
    );
}

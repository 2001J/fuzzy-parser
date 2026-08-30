use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum HeaderSelection {
    #[default]
    Automatic,
    None,
    Row(usize),
    SchemaSearch {
        max_rows: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InclusiveRowRange {
    pub start: usize,
    pub end: usize,
}

impl InclusiveRowRange {
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    fn contains(self, row: usize) -> bool {
        self.start <= row && row <= self.end
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RowSelection {
    pub include: Vec<InclusiveRowRange>,
    pub exclude: Vec<InclusiveRowRange>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SheetSelector {
    Name(String),
    Index(usize),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum SheetSelection {
    #[default]
    All,
    Selected(Vec<SheetSelector>),
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TableSelectionOptions {
    pub header: HeaderSelection,
    pub rows: RowSelection,
    pub sheets: SheetSelection,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct TableByteSpan {
    pub byte_start: usize,
    pub byte_end: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UnsupportedTableMetadata {
    MergedRegion {
        start_row: usize,
        start_column: usize,
        end_row: usize,
        end_column: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableInventoryRow {
    pub source_row: usize,
    pub block_indices: Vec<usize>,
    pub blank: bool,
    pub byte_span: Option<TableByteSpan>,
    pub line_start: Option<usize>,
    pub line_end: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableInventorySheet {
    pub original_index: usize,
    pub name: Option<String>,
    pub rows: Vec<TableInventoryRow>,
    pub unsupported_metadata: Vec<UnsupportedTableMetadata>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableInventory {
    pub source_type: SourceType,
    pub sheets: Vec<TableInventorySheet>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TableRowRole {
    Parsed,
    Header,
    Preamble,
    Excluded,
    Unselected,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TableRowEvidence {
    pub source_row: usize,
    pub block_indices: Vec<usize>,
    pub blank: bool,
    pub role: TableRowRole,
    pub reason: Option<Reason>,
    pub byte_span: Option<TableByteSpan>,
    pub line_start: Option<usize>,
    pub line_end: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TableSheetEvidence {
    pub original_index: usize,
    pub sheet: Option<String>,
    pub selection_order: Option<usize>,
    pub rows: Vec<TableRowEvidence>,
    pub unsupported_metadata: Vec<UnsupportedTableMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TableSourceEvidence {
    pub sheets: Vec<TableSheetEvidence>,
}

pub fn table_selection_failure(reason: TableSelectionReason, sheet: Option<&str>) -> Failure {
    Failure::new(FailureKind::TableSelection { reason }).with_context(DiagnosticContext {
        sheet: sheet.map(str::to_owned),
        ..DiagnosticContext::default()
    })
}

pub fn parse_document_with_plan_and_table_selection(
    document: &RawDocument,
    plan: &ParsePlan,
    inventory: &TableInventory,
    options: &TableSelectionOptions,
) -> Result<ParseResponse, Failure> {
    if !matches!(inventory.source_type, SourceType::Csv | SourceType::Xlsx)
        || document.source.source_type != inventory.source_type
    {
        return Err(table_selection_failure(
            TableSelectionReason::UnsupportedSource,
            None,
        ));
    }
    validate_ranges(&options.rows.include)?;
    validate_ranges(&options.rows.exclude)?;
    let selected = select_sheets(inventory, &options.sheets)?;
    let mut warnings = document.warnings.clone();
    let mut output_sheets = Vec::new();
    let mut roles = inventory
        .sheets
        .iter()
        .map(|sheet| {
            sheet
                .rows
                .iter()
                .map(|_| TableRowRole::Unselected)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    for (selection_order, sheet_index) in selected.iter().copied().enumerate() {
        let sheet = &inventory.sheets[sheet_index];
        validate_ranges_for_sheet(&options.rows, sheet)?;
        let rows = sheet
            .rows
            .iter()
            .map(|row| row_group(document, sheet, row))
            .collect::<Result<Vec<_>, _>>()?;
        let (header, mut header_warnings) = select_header(&rows, plan, &options.header)?;
        warnings.append(&mut header_warnings);
        let header_row = header.context().map(|context| context.source_row);
        if let HeaderSelection::Row(row) = options.header
            && options
                .rows
                .include
                .iter()
                .chain(&options.rows.exclude)
                .any(|range| range.contains(row))
        {
            return Err(table_selection_failure(
                TableSelectionReason::HeaderConflict,
                sheet.name.as_deref(),
            ));
        }

        let mut records = Vec::new();
        for (row_index, row) in rows.iter().enumerate() {
            let role = if Some(row.source_row) == header_row {
                TableRowRole::Header
            } else if header_row.is_some_and(|header| row.source_row < header) {
                TableRowRole::Preamble
            } else {
                let included = options.rows.include.is_empty()
                    || options
                        .rows
                        .include
                        .iter()
                        .any(|range| range.contains(row.source_row));
                let excluded = options
                    .rows
                    .exclude
                    .iter()
                    .any(|range| range.contains(row.source_row));
                if included && !excluded {
                    records.push(parse_row_group(
                        row,
                        header.context(),
                        plan.fields(),
                        DetectionRules::Scoped(plan.enum_definitions()),
                    ));
                    TableRowRole::Parsed
                } else {
                    TableRowRole::Excluded
                }
            };
            roles[sheet_index][row_index] = role;
        }
        if !sheet.unsupported_metadata.is_empty() {
            warnings.push(ParserWarning {
                code: "merged_regions_unsupported".to_owned(),
                message: "merged regions were detected and retained as unsupported metadata without interpretation".to_owned(),
                location: Some(SourceLocation {
                    sheet: sheet.name.clone(),
                    ..SourceLocation::default()
                }),
            });
        }
        output_sheets.push(SheetTableResult {
            sheet: sheet.name.clone(),
            header,
            records,
        });
        debug_assert_eq!(selection_order + 1, output_sheets.len());
    }

    let content = ParseContent::Table {
        sheets: output_sheets,
    };
    let table_evidence = build_evidence(inventory, &selected, &roles);
    let mut source_evidence = SourceEvidence::new(document, &content);
    apply_table_coverage(&mut source_evidence, &table_evidence);
    source_evidence.table = Some(table_evidence);
    Ok(ParseResponse {
        contract_version: CONTRACT_VERSION.to_owned(),
        parser_version: env!("CARGO_PKG_VERSION").to_owned(),
        record_name: plan.record_name(),
        source_type: document.source.source_type.clone(),
        content,
        warnings,
        source_evidence: Some(source_evidence),
    })
}

fn validate_ranges(ranges: &[InclusiveRowRange]) -> Result<(), Failure> {
    for range in ranges {
        if range.start == 0 || range.start > range.end {
            return Err(table_selection_failure(
                TableSelectionReason::InvalidRowRange,
                None,
            ));
        }
    }
    let mut sorted = ranges.to_vec();
    sorted.sort_by_key(|range| (range.start, range.end));
    if sorted.windows(2).any(|pair| pair[1].start <= pair[0].end) {
        return Err(table_selection_failure(
            TableSelectionReason::OverlappingRowRange,
            None,
        ));
    }
    Ok(())
}

fn validate_ranges_for_sheet(
    selection: &RowSelection,
    sheet: &TableInventorySheet,
) -> Result<(), Failure> {
    for range in selection.include.iter().chain(&selection.exclude) {
        if !sheet.rows.iter().any(|row| range.contains(row.source_row)) {
            return Err(table_selection_failure(
                TableSelectionReason::RowNotFound,
                sheet.name.as_deref(),
            ));
        }
    }
    Ok(())
}

fn select_sheets(
    inventory: &TableInventory,
    selection: &SheetSelection,
) -> Result<Vec<usize>, Failure> {
    match selection {
        SheetSelection::All => {
            let mut indices = (0..inventory.sheets.len()).collect::<Vec<_>>();
            indices.sort_by(|left, right| {
                inventory.sheets[*left]
                    .name
                    .cmp(&inventory.sheets[*right].name)
            });
            Ok(indices)
        }
        SheetSelection::Selected(selectors) => {
            if selectors.is_empty() {
                return Err(table_selection_failure(
                    TableSelectionReason::EmptySheetSelection,
                    None,
                ));
            }
            if inventory.source_type != SourceType::Xlsx {
                return Err(table_selection_failure(
                    TableSelectionReason::UnsupportedSource,
                    None,
                ));
            }
            let mut selected = Vec::new();
            for selector in selectors {
                let index = match selector {
                    SheetSelector::Name(name) => inventory
                        .sheets
                        .iter()
                        .position(|sheet| sheet.name.as_deref() == Some(name))
                        .ok_or_else(|| {
                            table_selection_failure(TableSelectionReason::MissingSheet, Some(name))
                        })?,
                    SheetSelector::Index(index) => inventory
                        .sheets
                        .iter()
                        .position(|sheet| sheet.original_index == *index)
                        .ok_or_else(|| {
                            table_selection_failure(
                                TableSelectionReason::SheetIndexOutOfRange,
                                None,
                            )
                        })?,
                };
                if selected.contains(&index) {
                    return Err(table_selection_failure(
                        TableSelectionReason::DuplicateSheetSelection,
                        inventory.sheets[index].name.as_deref(),
                    ));
                }
                selected.push(index);
            }
            Ok(selected)
        }
    }
}

fn row_group(
    document: &RawDocument,
    sheet: &TableInventorySheet,
    inventory: &TableInventoryRow,
) -> Result<TableRowGroup, Failure> {
    let mut cells = Vec::new();
    let mut source_block_ids = Vec::new();
    for &block_index in &inventory.block_indices {
        let Some(block) = document.blocks.get(block_index) else {
            return Err(table_selection_failure(
                TableSelectionReason::RowNotFound,
                sheet.name.as_deref(),
            ));
        };
        cells.push(TableCell {
            source_column: block.location.column.unwrap_or(0),
            value: block.value.clone(),
            source_block_id: block.id.clone(),
            source_block_index: Some(block_index),
        });
        source_block_ids.push(block.id.clone());
    }
    cells.sort_by_key(|cell| cell.source_column);
    Ok(TableRowGroup {
        sheet: sheet.name.clone(),
        source_row: inventory.source_row,
        cells,
        source_block_ids,
    })
}

fn select_header(
    rows: &[TableRowGroup],
    plan: &ParsePlan,
    selection: &HeaderSelection,
) -> Result<(HeaderExtraction, Vec<ParserWarning>), Failure> {
    match selection {
        HeaderSelection::Automatic => Ok((detect_table_headers(rows), Vec::new())),
        HeaderSelection::None => Ok((
            HeaderExtraction::not_detected(
                "header_disabled",
                "header interpretation was disabled by the caller",
            ),
            Vec::new(),
        )),
        HeaderSelection::Row(source_row) => {
            let row = rows
                .iter()
                .find(|row| row.source_row == *source_row)
                .ok_or_else(|| {
                    table_selection_failure(
                        TableSelectionReason::HeaderNotFound,
                        rows.first().and_then(|row| row.sheet.as_deref()),
                    )
                })?;
            Ok((explicit_header(row), Vec::new()))
        }
        HeaderSelection::SchemaSearch { max_rows } => {
            if *max_rows == 0 {
                return Err(table_selection_failure(
                    TableSelectionReason::HeaderNotFound,
                    rows.first().and_then(|row| row.sheet.as_deref()),
                ));
            }
            let scored = rows
                .iter()
                .take(*max_rows)
                .map(|row| (row, schema_header_score(row, plan.fields())))
                .collect::<Vec<_>>();
            let best = scored.iter().map(|(_, score)| *score).max().unwrap_or(0);
            let winners = scored
                .iter()
                .filter(|(_, score)| *score == best && best > 0)
                .collect::<Vec<_>>();
            if let [winner] = winners.as_slice() {
                return Ok((explicit_header(winner.0), Vec::new()));
            }
            let tied = winners.len() > 1;
            let code = if tied {
                "header_search_ambiguous"
            } else {
                "header_search_no_match"
            };
            let message = if tied {
                "schema-informed header search found multiple equally best rows"
            } else {
                "schema-informed header search found no positive row match"
            };
            Ok((
                HeaderExtraction::NotDetected {
                    code: code.to_owned(),
                    message: message.to_owned(),
                },
                vec![ParserWarning {
                    code: code.to_owned(),
                    message: message.to_owned(),
                    location: rows.first().map(|row| SourceLocation {
                        sheet: row.sheet.clone(),
                        ..SourceLocation::default()
                    }),
                }],
            ))
        }
    }
}

fn explicit_header(row: &TableRowGroup) -> HeaderExtraction {
    HeaderExtraction::Detected {
        headers: Box::new(TableHeaderContext {
            sheet: row.sheet.clone(),
            source_row: row.source_row,
            labels: row
                .cells
                .iter()
                .map(|cell| {
                    (
                        cell.source_column,
                        collapse_whitespace(cell.value.to_text().trim()),
                    )
                })
                .collect(),
            source_block_ids: row.source_block_ids.clone(),
        }),
    }
}

fn schema_header_score(row: &TableRowGroup, fields: &[AssignmentField]) -> usize {
    fields
        .iter()
        .filter(|field| {
            row.cells.iter().any(|cell| {
                let label = collapse_whitespace(cell.value.to_text().trim());
                std::iter::once(field.name.as_str())
                    .chain(field.aliases.iter().map(String::as_str))
                    .any(|candidate| candidate.eq_ignore_ascii_case(&label))
            })
        })
        .count()
}

fn build_evidence(
    inventory: &TableInventory,
    selected: &[usize],
    roles: &[Vec<TableRowRole>],
) -> TableSourceEvidence {
    TableSourceEvidence {
        sheets: inventory
            .sheets
            .iter()
            .enumerate()
            .map(|(sheet_index, sheet)| TableSheetEvidence {
                original_index: sheet.original_index,
                sheet: sheet.name.clone(),
                selection_order: selected
                    .iter()
                    .position(|selected| *selected == sheet_index)
                    .map(|index| index + 1),
                rows: sheet
                    .rows
                    .iter()
                    .zip(&roles[sheet_index])
                    .map(|(row, role)| TableRowEvidence {
                        source_row: row.source_row,
                        block_indices: row.block_indices.clone(),
                        blank: row.blank,
                        role: *role,
                        reason: row_reason(*role),
                        byte_span: row.byte_span,
                        line_start: row.line_start,
                        line_end: row.line_end,
                    })
                    .collect(),
                unsupported_metadata: sheet.unsupported_metadata.clone(),
            })
            .collect(),
    }
}

fn row_reason(role: TableRowRole) -> Option<Reason> {
    match role {
        TableRowRole::Parsed => None,
        TableRowRole::Header => Some(Reason::new(
            "header_selected",
            "the row was selected as header context, not record content",
        )),
        TableRowRole::Preamble => Some(Reason::new(
            "preamble_before_header",
            "the row precedes the selected header and was retained as preamble evidence",
        )),
        TableRowRole::Excluded => Some(Reason::new(
            "row_excluded",
            "the row was excluded by caller-provided row selection",
        )),
        TableRowRole::Unselected => Some(Reason::new(
            "sheet_unselected",
            "the row belongs to a sheet that was not selected",
        )),
    }
}

fn apply_table_coverage(evidence: &mut SourceEvidence, table: &TableSourceEvidence) {
    for sheet in &table.sheets {
        for row in &sheet.rows {
            let block_role = match row.role {
                TableRowRole::Parsed => SourceBlockRole::Parsed,
                TableRowRole::Header => SourceBlockRole::Header,
                TableRowRole::Preamble | TableRowRole::Excluded | TableRowRole::Unselected => {
                    SourceBlockRole::Excluded
                }
            };
            for &block_index in &row.block_indices {
                if let Some(block) = evidence.blocks.get_mut(block_index) {
                    block.role = block_role;
                    block.reason = row.reason.clone();
                    if block_role != SourceBlockRole::Parsed {
                        block.unused_spans.clear();
                    }
                }
            }
        }
    }
}

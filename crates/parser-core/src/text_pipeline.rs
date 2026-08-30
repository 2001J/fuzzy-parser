use super::*;

pub(super) struct ComposedRecord {
    pub source_block_id: String,
    pub composition: TextRecordComposition,
}

#[derive(Clone)]
struct MappingUnit {
    raw_span: TextSpan,
    text: String,
    operations: Vec<TextMappingOperation>,
}

pub(super) struct MappedNormalization {
    pub normalized_text: String,
    units: Vec<MappingUnit>,
    pub transformations: Vec<Transformation>,
}

struct MappedBlock {
    block_index: usize,
    coordinate_space: SourceCoordinateSpace,
    normalized_text: String,
    units: Vec<MappingUnit>,
    transformations: Vec<Transformation>,
}

struct SourcePart {
    block_index: usize,
    coordinate_space: SourceCoordinateSpace,
    raw_span: TextSpan,
    text: String,
    runs: Vec<TextMappingRun>,
}

enum RepeatedOutcome {
    Split(Vec<TextSpan>),
    NoEvidence,
    Ambiguous,
}

pub(super) fn compose_document(
    document: &RawDocument,
    options: &TextPipelineOptions,
) -> Vec<ComposedRecord> {
    let blocks: Vec<_> = document
        .blocks
        .iter()
        .enumerate()
        .map(|(index, block)| mapped_block(index, block, &options.normalization))
        .collect();
    let specs = match options.strategy {
        TextPipelineStrategy::OneBlockPerRecord => blocks
            .iter()
            .enumerate()
            .map(|(index, block)| {
                (
                    vec![whole_part(block)],
                    TextBoundaryEvidence {
                        confidence: 1.0,
                        reasons: vec![Reason::new(
                            "one_block_boundary",
                            "one source block is one record",
                        )],
                        warnings: Vec::new(),
                    },
                    index,
                )
            })
            .collect(),
        TextPipelineStrategy::JoinIndentedContinuations => joined_specs(document, &blocks),
        TextPipelineStrategy::SplitRepeatedIdentifiers => {
            repeated_specs(document, &blocks, &options.repeated_identifier_markers)
        }
    };

    specs
        .into_iter()
        .enumerate()
        .map(|(record_index, (parts, boundary, first_block_index))| {
            let mut composed_text = String::new();
            let mut segments = Vec::new();
            for (part_index, part) in parts.into_iter().enumerate() {
                if part_index > 0 {
                    let start = composed_text.len();
                    composed_text.push('\n');
                    segments.push(TextCompositionSegment::SyntheticSeparator {
                        composed_span: TextSpan {
                            byte_start: start,
                            byte_end: composed_text.len(),
                        },
                    });
                }
                let offset = composed_text.len();
                composed_text.push_str(&part.text);
                let composed_span = TextSpan {
                    byte_start: offset,
                    byte_end: composed_text.len(),
                };
                let mapping_runs = part
                    .runs
                    .into_iter()
                    .map(|mut run| {
                        run.composed_span.byte_start += offset;
                        run.composed_span.byte_end += offset;
                        run
                    })
                    .collect();
                segments.push(TextCompositionSegment::Source {
                    source_reference: SourceReference {
                        block_index: part.block_index,
                        coordinate_space: part.coordinate_space,
                        span: part.raw_span,
                    },
                    composed_span,
                    mapping_runs,
                });
            }
            ComposedRecord {
                source_block_id: document.blocks[first_block_index].id.clone(),
                composition: TextRecordComposition {
                    record_id: format!("record-{}", record_index + 1),
                    composed_text,
                    applied_options: options.clone(),
                    segments,
                    boundary,
                },
            }
        })
        .collect()
}

pub(super) fn parse_record(
    document: &RawDocument,
    composition: &TextRecordComposition,
    fields: &[AssignmentField],
    rules: DetectionRules<'_>,
) -> TextParseResult {
    let text = &composition.composed_text;
    let mut candidates = Vec::new();
    let mut sources = Vec::new();
    let mut source_spans = Vec::new();
    for segment in &composition.segments {
        let TextCompositionSegment::Source {
            source_reference,
            composed_span,
            ..
        } = segment
        else {
            continue;
        };
        source_spans.push(composed_span.clone());
        let local = &text[composed_span.byte_start..composed_span.byte_end];
        for mut candidate in collect_field_candidates(local, rules) {
            candidate.source_span.byte_start += composed_span.byte_start;
            candidate.source_span.byte_end += composed_span.byte_start;
            candidates.push(candidate);
        }
        if matches!(
            document.blocks[source_reference.block_index].value,
            RawValue::Text(_)
        ) {
            sources.push(text_fields::TextSource {
                span: composed_span.clone(),
                reference: None,
                column: None,
            });
        }
    }
    let mut assignment = assign_scalar_candidates(
        text,
        &candidates,
        fields,
        None,
        rules.scoped(),
        Some(&source_spans),
    );
    let mut prepare = |candidate: &mut FieldCandidate| {
        let _ = rehydrate_candidate(document, composition, candidate);
    };
    text_fields::complete(
        text,
        &mut candidates,
        fields,
        None,
        &sources,
        &mut assignment,
        Some(&mut prepare),
    );
    rehydrate_all(document, composition, &mut candidates, &mut assignment);
    let mut parse = finish_text_parse(text, candidates, assignment);
    if !composition.boundary.warnings.is_empty() {
        let review = parse.review.get_or_insert(RecordReview {
            status: RecordReviewStatus::NeedsReview,
            reasons: Vec::new(),
        });
        review.status = RecordReviewStatus::NeedsReview;
        review.reasons.push(Reason::new(
            "composition_boundary_warnings",
            "record composition produced boundary warnings",
        ));
    }
    parse
}

fn rehydrate_all(
    document: &RawDocument,
    composition: &TextRecordComposition,
    candidates: &mut [FieldCandidate],
    assignment: &mut AssignmentResult,
) {
    let mut failed = Vec::new();
    for candidate in candidates.iter_mut() {
        if !rehydrate_candidate(document, composition, candidate) {
            failed.push((
                candidate.candidate_type.clone(),
                candidate.source_span.clone(),
            ));
        }
    }
    let mut removed = Vec::new();
    for field in &mut assignment.fields {
        let mut retained = Vec::new();
        for mut candidate in field.candidates.drain(..) {
            if failed.contains(&(
                candidate.candidate_type.clone(),
                candidate.source_span.clone(),
            )) {
                candidate.source_reference = None;
                removed.push(candidate);
            } else {
                let _ = rehydrate_candidate(document, composition, &mut candidate);
                retained.push(candidate);
            }
        }
        field.candidates = retained;
    }
    assignment
        .fields
        .retain(|field| !field.candidates.is_empty());
    assignment.unassigned_candidates.extend(removed);
    for candidate in &mut assignment.unassigned_candidates {
        let _ = rehydrate_candidate(document, composition, candidate);
    }
    if !failed.is_empty() {
        assignment.warnings.push(ParserWarning {
            code: "source_mapping_unresolved".to_owned(),
            message: "candidate evidence could not be mapped to one contiguous original source span and remains unresolved".to_owned(),
            location: None,
        });
    }
}

fn rehydrate_candidate(
    document: &RawDocument,
    composition: &TextRecordComposition,
    candidate: &mut FieldCandidate,
) -> bool {
    let Some(reference) = map_span(composition, &candidate.source_span) else {
        candidate.source_reference = None;
        return false;
    };
    let Some(raw) = reference.resolve(document) else {
        candidate.source_reference = None;
        return false;
    };
    candidate.raw_value = raw.clone();
    if matches!(
        candidate.candidate_type,
        CandidateType::Text | CandidateType::PersonName
    ) {
        candidate.normalized_value = Some(serde_json::Value::String(raw));
    }
    candidate.source_reference = Some(reference);
    true
}

fn map_span(composition: &TextRecordComposition, span: &TextSpan) -> Option<SourceReference> {
    for segment in &composition.segments {
        let TextCompositionSegment::Source {
            source_reference,
            composed_span,
            mapping_runs,
        } = segment
        else {
            continue;
        };
        if span.byte_start < composed_span.byte_start
            || span.byte_end > composed_span.byte_end
            || span.byte_start >= span.byte_end
        {
            continue;
        }
        let positive: Vec<_> = mapping_runs
            .iter()
            .filter(|run| {
                run.composed_span.byte_start < span.byte_end
                    && span.byte_start < run.composed_span.byte_end
            })
            .collect();
        let (Some(first), Some(last)) = (positive.first(), positive.last()) else {
            return None;
        };
        if first.composed_span.byte_start != span.byte_start
            || last.composed_span.byte_end != span.byte_end
            || positive
                .windows(2)
                .any(|runs| runs[0].raw_span.byte_end != runs[1].raw_span.byte_start)
        {
            return None;
        }
        return Some(SourceReference {
            block_index: source_reference.block_index,
            coordinate_space: source_reference.coordinate_space,
            span: TextSpan {
                byte_start: first.raw_span.byte_start,
                byte_end: last.raw_span.byte_end,
            },
        });
    }
    None
}

fn mapped_block(
    block_index: usize,
    block: &RawBlock,
    options: &NormalizationOptions,
) -> MappedBlock {
    let raw = block.value.to_text();
    let mapped = normalize_with_mapping(&raw, options);
    MappedBlock {
        block_index,
        coordinate_space: SourceCoordinateSpace::for_value(&block.value),
        normalized_text: mapped.normalized_text,
        units: mapped.units,
        transformations: mapped.transformations,
    }
}

pub(super) fn normalize_with_mapping(
    raw: &str,
    options: &NormalizationOptions,
) -> MappedNormalization {
    let mut units = initial_units(raw);
    let mut transformations = Vec::new();
    if options.normalize_line_endings {
        let changed = units.iter().any(|unit| unit.text == "\r");
        units = fold_line_endings(units);
        if changed {
            transformations.push(Transformation::LineEndingsNormalized);
        }
    }
    if options.normalize_punctuation {
        let (dash_changed, quote_changed) = replace_punctuation(&mut units);
        if dash_changed {
            transformations.push(Transformation::DashesNormalized);
        }
        if quote_changed {
            transformations.push(Transformation::QuotesNormalized);
        }
    }
    if options.trim_whitespace && trim_units(&mut units) {
        transformations.push(Transformation::WhitespaceTrimmed);
    }
    if options.collapse_whitespace {
        let before: String = units.iter().map(|unit| unit.text.as_str()).collect();
        units = collapse_units(units);
        let after: String = units.iter().map(|unit| unit.text.as_str()).collect();
        if before != after {
            transformations.push(Transformation::WhitespaceCollapsed);
        }
    }
    let normalized_text: String = units.iter().map(|unit| unit.text.as_str()).collect();
    if options.mark_noise {
        mark_noise(&normalized_text, &mut transformations);
    }
    MappedNormalization {
        normalized_text,
        units,
        transformations,
    }
}

fn initial_units(raw: &str) -> Vec<MappingUnit> {
    raw.char_indices()
        .map(|(start, character)| MappingUnit {
            raw_span: TextSpan {
                byte_start: start,
                byte_end: start + character.len_utf8(),
            },
            text: character.to_string(),
            operations: Vec::new(),
        })
        .collect()
}

fn fold_line_endings(units: Vec<MappingUnit>) -> Vec<MappingUnit> {
    let mut output = Vec::new();
    let mut iter = units.into_iter().peekable();
    while let Some(mut unit) = iter.next() {
        if unit.text == "\r" {
            if iter.peek().is_some_and(|next| next.text == "\n") {
                let next = iter.next().expect("peeked line-feed exists");
                unit.raw_span.byte_end = next.raw_span.byte_end;
            }
            unit.text = "\n".to_owned();
            add_operation(&mut unit.operations, TextMappingOperation::LineEndingFold);
        }
        output.push(unit);
    }
    output
}

fn replace_punctuation(units: &mut [MappingUnit]) -> (bool, bool) {
    let mut dash_changed = false;
    let mut quote_changed = false;
    for unit in units {
        let replacement = match unit.text.as_str() {
            "\u{2010}" | "\u{2011}" | "\u{2012}" | "\u{2013}" | "\u{2014}" | "\u{2212}" => {
                dash_changed = true;
                Some("-")
            }
            "\u{2018}" | "\u{2019}" => {
                quote_changed = true;
                Some("'")
            }
            "\u{201c}" | "\u{201d}" => {
                quote_changed = true;
                Some("\"")
            }
            _ => None,
        };
        if let Some(replacement) = replacement {
            unit.text = replacement.to_owned();
            add_operation(
                &mut unit.operations,
                TextMappingOperation::PunctuationReplacement,
            );
        }
    }
    (dash_changed, quote_changed)
}

fn trim_units(units: &mut [MappingUnit]) -> bool {
    let start = units
        .iter()
        .position(|unit| !unit.text.chars().all(char::is_whitespace))
        .unwrap_or(units.len());
    let end = units
        .iter()
        .rposition(|unit| !unit.text.chars().all(char::is_whitespace))
        .map_or(start, |index| index + 1);
    let (leading, rest) = units.split_at_mut(start);
    let trailing = &mut rest[end.saturating_sub(start)..];
    let changed = leading
        .iter()
        .chain(trailing.iter())
        .any(|unit| !unit.text.is_empty());
    for unit in leading.iter_mut().chain(trailing.iter_mut()) {
        unit.text.clear();
        add_operation(&mut unit.operations, TextMappingOperation::Trim);
    }
    changed
}

fn collapse_units(units: Vec<MappingUnit>) -> Vec<MappingUnit> {
    let mut output: Vec<MappingUnit> = Vec::new();
    for mut unit in units {
        if unit.text.is_empty() {
            output.push(unit);
            continue;
        }
        if unit.text.chars().all(char::is_whitespace) {
            if let Some(previous) = output.last_mut()
                && previous.text == " "
                && previous.raw_span.byte_end == unit.raw_span.byte_start
            {
                previous.raw_span.byte_end = unit.raw_span.byte_end;
                add_operation(
                    &mut previous.operations,
                    TextMappingOperation::WhitespaceCollapse,
                );
                for operation in unit.operations {
                    add_operation(&mut previous.operations, operation);
                }
                continue;
            }
            if unit.text != " " {
                unit.text = " ".to_owned();
                add_operation(
                    &mut unit.operations,
                    TextMappingOperation::WhitespaceCollapse,
                );
            }
        }
        output.push(unit);
    }
    output
}

fn add_operation(operations: &mut Vec<TextMappingOperation>, operation: TextMappingOperation) {
    operations.retain(|existing| *existing != TextMappingOperation::Unchanged);
    if !operations.contains(&operation) {
        operations.push(operation);
    }
}

fn whole_part(block: &MappedBlock) -> SourcePart {
    part_from_units(block, 0, block.normalized_text.len(), true)
        .expect("a complete mapped block is always representable")
}

fn part_from_units(
    block: &MappedBlock,
    start: usize,
    end: usize,
    include_zero_edges: bool,
) -> Option<SourcePart> {
    let mut local_offset = 0;
    let mut selected = Vec::new();
    for unit in &block.units {
        let unit_start = local_offset;
        let unit_end = unit_start + unit.text.len();
        local_offset = unit_end;
        let positive_overlap = unit_start < end && start < unit_end;
        let zero_inside = unit_start == unit_end
            && ((include_zero_edges && start <= unit_start && unit_start <= end)
                || (start < unit_start && unit_start < end));
        if positive_overlap || zero_inside {
            if positive_overlap && (unit_start < start || unit_end > end) {
                return None;
            }
            selected.push((unit, unit_start, unit_end));
        }
    }
    if selected.is_empty() && start != end {
        return None;
    }
    let raw_span = if let (Some(first), Some(last)) = (selected.first(), selected.last()) {
        TextSpan {
            byte_start: first.0.raw_span.byte_start,
            byte_end: last.0.raw_span.byte_end,
        }
    } else {
        TextSpan {
            byte_start: 0,
            byte_end: 0,
        }
    };
    let runs = selected
        .into_iter()
        .map(|(unit, unit_start, unit_end)| TextMappingRun {
            raw_span: unit.raw_span.clone(),
            composed_span: TextSpan {
                byte_start: unit_start.saturating_sub(start),
                byte_end: unit_end.saturating_sub(start),
            },
            operations: if unit.operations.is_empty() {
                vec![TextMappingOperation::Unchanged]
            } else {
                unit.operations.clone()
            },
        })
        .collect();
    Some(SourcePart {
        block_index: block.block_index,
        coordinate_space: block.coordinate_space,
        raw_span,
        text: block.normalized_text[start..end].to_owned(),
        runs,
    })
}

type RecordSpec = (Vec<SourcePart>, TextBoundaryEvidence, usize);

fn joined_specs(document: &RawDocument, blocks: &[MappedBlock]) -> Vec<RecordSpec> {
    let mut groups: Vec<Vec<usize>> = Vec::new();
    for (index, block) in blocks.iter().enumerate() {
        let previous = groups.last().and_then(|group| group.last()).copied();
        let joins = !block.normalized_text.is_empty()
            && has_leading_whitespace(&document.blocks[index].value)
            && previous.is_some()
            && !is_heading(block)
            && !previous.is_some_and(|previous| is_heading(&blocks[previous]));
        if joins {
            groups.last_mut().expect("group exists").push(index);
        } else {
            groups.push(vec![index]);
        }
    }
    groups
        .into_iter()
        .map(|group| {
            let first = group[0];
            let heading_warning = if group.len() == 1
                && has_leading_whitespace(&document.blocks[first].value)
                && first
                    .checked_sub(1)
                    .is_some_and(|previous| is_heading(&blocks[previous]))
            {
                Some(ParserWarning {
                    code: "ambiguous_heading_continuation".to_owned(),
                    message: "the block was kept separate because an indented continuation after a heading has an ambiguous record boundary".to_owned(),
                    location: Some(document.blocks[first].location.clone()),
                })
            } else {
                None
            };
            let (confidence, reason) = if heading_warning.is_some() {
                (
                    0.35,
                    Reason::new(
                        "ambiguous_heading_continuation",
                        "indented text after a heading was kept separate because its record boundary is ambiguous",
                    ),
                )
            } else if group.len() == 1 && is_heading(&blocks[first]) {
                (
                    0.98,
                    Reason::new(
                        "heading_boundary",
                        "a heading-marked source block starts a visible section boundary",
                    ),
                )
            } else if group.len() > 1 {
                (
                    0.85,
                    Reason::new(
                        "indented_continuation",
                        "indented source blocks were joined to the preceding block",
                    ),
                )
            } else {
                (
                    1.0,
                    Reason::new("record_start", "no continuation evidence joined this block"),
                )
            };
            (
                group.iter().map(|index| whole_part(&blocks[*index])).collect(),
                TextBoundaryEvidence {
                    confidence,
                    reasons: vec![reason],
                    warnings: heading_warning.into_iter().collect(),
                },
                first,
            )
        })
        .collect()
}

fn repeated_specs(
    document: &RawDocument,
    blocks: &[MappedBlock],
    markers: &[String],
) -> Vec<RecordSpec> {
    let mut specs = Vec::new();
    for block in blocks {
        match repeated_outcome(&block.normalized_text, markers) {
            RepeatedOutcome::Split(spans) => {
                for span in spans {
                    let part = part_from_units(block, span.byte_start, span.byte_end, false)
                        .expect("validated split boundaries follow mapped scalar boundaries");
                    specs.push((
                        vec![part],
                        TextBoundaryEvidence {
                            confidence: 0.82,
                            reasons: vec![Reason::new(
                                "repeated_identifier_boundary",
                                "a repeated strong identifier marker established a record boundary",
                            )],
                            warnings: Vec::new(),
                        },
                        block.block_index,
                    ));
                }
            }
            RepeatedOutcome::NoEvidence => specs.push((
                vec![whole_part(block)],
                TextBoundaryEvidence {
                    confidence: 0.9,
                    reasons: vec![Reason::new(
                        "no_repeated_identifier_boundary",
                        "no repeated strong identifier marker established a record boundary",
                    )],
                    warnings: Vec::new(),
                },
                block.block_index,
            )),
            RepeatedOutcome::Ambiguous => specs.push((
                vec![whole_part(block)],
                TextBoundaryEvidence {
                    confidence: 0.35,
                    reasons: vec![Reason::new(
                        "ambiguous_repeated_identifier_boundary",
                        "repeated identifier evidence did not establish a safe record boundary",
                    )],
                    warnings: vec![ParserWarning {
                        code: "ambiguous_repeated_identifier_boundary".to_owned(),
                        message: "the block was kept intact because repeated identifier evidence was ambiguous".to_owned(),
                        location: Some(document.blocks[block.block_index].location.clone()),
                    }],
                },
                block.block_index,
            )),
        }
    }
    specs
}

fn repeated_outcome(text: &str, markers: &[String]) -> RepeatedOutcome {
    let mut by_marker: Vec<(String, Vec<usize>)> = Vec::new();
    let lowered = text.to_ascii_lowercase();
    for marker in markers {
        let marker_lower = marker.to_ascii_lowercase();
        let mut search = 0;
        while search < lowered.len() {
            let Some(relative) = lowered[search..].find(&marker_lower) else {
                break;
            };
            let start = search + relative;
            let end = start + marker_lower.len();
            if is_strong_identifier_occurrence(text, start, end) {
                if let Some((_, positions)) = by_marker
                    .iter_mut()
                    .find(|(value, _)| value == &marker_lower)
                {
                    positions.push(start);
                } else {
                    by_marker.push((marker_lower.clone(), vec![start]));
                }
            }
            search = end.max(start + 1);
        }
    }
    for (_, positions) in &mut by_marker {
        positions.sort_unstable();
        positions.dedup();
    }
    let repeated: Vec<_> = by_marker
        .into_iter()
        .filter(|(_, positions)| positions.len() > 1)
        .collect();
    if repeated.is_empty() {
        return RepeatedOutcome::NoEvidence;
    }
    if repeated.len() > 1 || repeated[0].1[0] != 0 {
        return RepeatedOutcome::Ambiguous;
    }
    let (marker, positions) = &repeated[0];
    let mut spans = Vec::new();
    for (index, start) in positions.iter().enumerate() {
        let end = positions.get(index + 1).copied().unwrap_or(text.len());
        let value_start = start + marker.len();
        if text[value_start..end].trim().is_empty() {
            return RepeatedOutcome::Ambiguous;
        }
        let part = &text[*start..end];
        let trimmed = part.trim();
        let trimmed_start = *start + part.len() - part.trim_start().len();
        spans.push(TextSpan {
            byte_start: trimmed_start,
            byte_end: trimmed_start + trimmed.len(),
        });
    }
    RepeatedOutcome::Split(spans)
}

fn is_heading(block: &MappedBlock) -> bool {
    block
        .transformations
        .contains(&Transformation::HeadingDetected)
}

#[cfg(test)]
pub(super) fn validate_composition(
    document: &RawDocument,
    composition: &TextRecordComposition,
) -> bool {
    let mut cursor = 0;
    for segment in &composition.segments {
        let span = match segment {
            TextCompositionSegment::Source {
                source_reference,
                composed_span,
                mapping_runs,
            } => {
                if source_reference.resolve(document).is_none() {
                    return false;
                }
                let mut raw_cursor = source_reference.span.byte_start;
                let mut composed_cursor = composed_span.byte_start;
                for run in mapping_runs {
                    if run.raw_span.byte_start != raw_cursor
                        || run.composed_span.byte_start != composed_cursor
                        || !document.blocks[source_reference.block_index]
                            .value
                            .to_text()
                            .is_char_boundary(run.raw_span.byte_start)
                        || !document.blocks[source_reference.block_index]
                            .value
                            .to_text()
                            .is_char_boundary(run.raw_span.byte_end)
                        || !composition
                            .composed_text
                            .is_char_boundary(run.composed_span.byte_start)
                        || !composition
                            .composed_text
                            .is_char_boundary(run.composed_span.byte_end)
                    {
                        return false;
                    }
                    raw_cursor = run.raw_span.byte_end;
                    composed_cursor = run.composed_span.byte_end;
                }
                if raw_cursor != source_reference.span.byte_end
                    || composed_cursor != composed_span.byte_end
                {
                    return false;
                }
                composed_span
            }
            TextCompositionSegment::SyntheticSeparator { composed_span } => composed_span,
        };
        if span.byte_start != cursor {
            return false;
        }
        cursor = span.byte_end;
    }
    cursor == composition.composed_text.len()
}

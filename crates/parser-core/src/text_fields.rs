use super::*;

pub(super) fn is_text(kind: &CandidateType) -> bool {
    matches!(kind, CandidateType::Text | CandidateType::PersonName)
}

/// A single original Text block/cell, before row flattening loses its boundary.
pub(super) struct TextSource {
    pub span: TextSpan,
    pub reference: Option<SourceReference>,
    pub column: Option<usize>,
}

struct Region {
    span: TextSpan,
    fields: Vec<usize>,
    reason: &'static str,
    ambiguous: bool,
}

struct Hypothesis {
    index: usize,
    owner: Option<usize>,
}

struct AnchorGroup {
    value_start: usize,
    syntax_start: usize,
    fields: Vec<usize>,
    ambiguous: bool,
}

fn intersects(a: &TextSpan, b: &TextSpan) -> bool {
    a.byte_start < b.byte_end && b.byte_start < a.byte_end
}

fn overlaps(a: &FieldCandidate, b: &FieldCandidate) -> bool {
    match (&a.source_reference, &b.source_reference) {
        (Some(a), Some(b)) => {
            a.block_index == b.block_index
                && a.coordinate_space == b.coordinate_space
                && intersects(&a.span, &b.span)
        }
        _ => intersects(&a.source_span, &b.source_span),
    }
}

fn warning(result: &mut AssignmentResult, code: &str) {
    let message = match code {
        "text_evidence_overlap" => {
            "a directed text region overlaps already assigned evidence; its remaining fragments are unresolved"
        }
        _ => {
            "text or name evidence has competing field ownership or singular values and remains unresolved"
        }
    };
    result.warnings.push(ParserWarning {
        code: code.into(),
        message: message.into(),
        location: None,
    });
}

fn trim_span(text: &str, span: &TextSpan) -> Option<TextSpan> {
    let value = text.get(span.byte_start..span.byte_end)?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    let start = span.byte_start + value.len() - value.trim_start().len();
    Some(TextSpan {
        byte_start: start,
        byte_end: start + trimmed.len(),
    })
}

fn candidate(
    text: &str,
    source: &TextSource,
    span: TextSpan,
    kind: CandidateType,
    directed: bool,
    reason: &'static str,
) -> FieldCandidate {
    let raw = text[span.byte_start..span.byte_end].to_owned();
    let mut reasons = vec![Reason::new(
        reason,
        match reason {
            "caller_label_match" => "the value follows a literal caller-provided field label",
            "header_label_match" => {
                "the value is in a column with a matching caller-provided header"
            }
            _ => "the contiguous text fragment is an unresolved hypothesis",
        },
    )];
    if kind == CandidateType::PersonName {
        reasons.push(Reason::new(
            "caller_person_name",
            "the caller requests a possible person name; this is not identity verification",
        ));
    }
    let reference = source.reference.as_ref().map(|reference| SourceReference {
        block_index: reference.block_index,
        coordinate_space: reference.coordinate_space,
        span: TextSpan {
            byte_start: reference.span.byte_start + span.byte_start - source.span.byte_start,
            byte_end: reference.span.byte_start + span.byte_end - source.span.byte_start,
        },
    });
    FieldCandidate {
        candidate_type: kind,
        raw_value: raw.clone(),
        normalized_value: Some(serde_json::Value::String(raw)),
        source_span: span,
        source_column: source.column,
        source_reference: reference,
        confidence: if directed { 0.80 } else { 0.30 },
        reasons,
    }
}

fn name_allowed(text: &str, span: &TextSpan, scalar: &[FieldCandidate]) -> bool {
    text[span.byte_start..span.byte_end]
        .chars()
        .any(char::is_alphabetic)
        && !scalar
            .iter()
            .any(|c| !is_text(&c.candidate_type) && c.source_span == *span)
}

/// Recognize literal labels, grouping overlapping anchors rather than choosing
/// a longest label or whichever field appears first in the schema.
fn regions(
    text: &str,
    source: &TextSource,
    fields: &[AssignmentField],
    header: Option<&TableHeaderContext>,
) -> (Vec<Region>, Vec<TextSpan>) {
    let header_owners: Vec<_> = fields
        .iter()
        .enumerate()
        .filter(|(_, field)| {
            is_text(&field.candidate_type)
                && header_label_for_column(header, source.column).is_some_and(|label| {
                    std::iter::once(field.name.as_str())
                        .chain(field.aliases.iter().map(String::as_str))
                        .any(|name| name.eq_ignore_ascii_case(label))
                })
        })
        .map(|(index, _)| index)
        .collect();
    if !header_owners.is_empty() {
        return (
            vec![Region {
                span: source.span.clone(),
                fields: header_owners,
                reason: "header_label_match",
                ambiguous: false,
            }],
            Vec::new(),
        );
    }
    let value = &text[source.span.byte_start..source.span.byte_end];
    let mut anchors = Vec::new();
    for (start, _) in value.char_indices() {
        if start > 0
            && !value[..start]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_whitespace() || matches!(c, ',' | ';'))
        {
            continue;
        }
        for (field_index, field) in fields.iter().enumerate() {
            for label in
                std::iter::once(field.name.as_str()).chain(field.aliases.iter().map(String::as_str))
            {
                let end = start + label.len();
                if value
                    .get(start..end)
                    .is_some_and(|s| s.eq_ignore_ascii_case(label))
                    && value.as_bytes().get(end) == Some(&b':')
                {
                    let mut syntax_start = start;
                    let prefix = value[..start].trim_end();
                    if prefix.ends_with([',', ';']) {
                        syntax_start = prefix.len() - 1;
                    }
                    anchors.push((start, end + 1, syntax_start, field_index));
                }
            }
        }
    }
    anchors.sort_unstable();
    anchors.dedup();
    let mut groups: Vec<AnchorGroup> = Vec::new();
    for (start, end, syntax, field) in anchors {
        if let Some(last) = groups.last_mut()
            && start < last.value_start
        {
            last.value_start = last.value_start.max(end);
            last.syntax_start = last.syntax_start.min(syntax);
            if !last.fields.contains(&field) {
                last.fields.push(field);
            }
            last.ambiguous = true;
        } else {
            groups.push(AnchorGroup {
                value_start: end,
                syntax_start: syntax,
                fields: vec![field],
                ambiguous: false,
            });
        }
    }
    let syntax = groups
        .iter()
        .map(|g| TextSpan {
            byte_start: source.span.byte_start + g.syntax_start,
            byte_end: source.span.byte_start + g.value_start,
        })
        .collect();
    let regions = groups
        .iter()
        .enumerate()
        .filter_map(|(i, g)| {
            let owners: Vec<_> = g
                .fields
                .iter()
                .copied()
                .filter(|index| is_text(&fields[*index].candidate_type))
                .collect();
            if owners.is_empty() {
                return None;
            }
            Some(Region {
                span: TextSpan {
                    byte_start: source.span.byte_start + g.value_start,
                    byte_end: source.span.byte_start
                        + groups
                            .get(i + 1)
                            .map_or(value.len(), |next| next.syntax_start),
                },
                fields: owners,
                reason: "caller_label_match",
                ambiguous: g.ambiguous,
            })
        })
        .collect();
    (regions, syntax)
}

fn gaps(span: &TextSpan, excluded: impl Iterator<Item = TextSpan>) -> Vec<TextSpan> {
    uncovered_spans(
        span.byte_end - span.byte_start,
        excluded
            .filter(|other| intersects(span, other))
            .map(|other| TextSpan {
                byte_start: other.byte_start.max(span.byte_start) - span.byte_start,
                byte_end: other.byte_end.min(span.byte_end) - span.byte_start,
            }),
    )
    .into_iter()
    .map(|gap| TextSpan {
        byte_start: gap.byte_start + span.byte_start,
        byte_end: gap.byte_end + span.byte_start,
    })
    .collect()
}

fn residual(
    text: &str,
    source: &TextSource,
    span: TextSpan,
    fields: &[AssignmentField],
    scalar: &[FieldCandidate],
    candidates: &mut Vec<FieldCandidate>,
    hypotheses: &mut Vec<Hypothesis>,
) -> bool {
    let Some(span) = trim_span(text, &span) else {
        return false;
    };
    if !text[span.byte_start..span.byte_end]
        .chars()
        .any(char::is_alphabetic)
    {
        return false;
    }
    let eligible = fields
        .iter()
        .filter(|field| {
            field.candidate_type == CandidateType::Text
                || (field.candidate_type == CandidateType::PersonName
                    && name_allowed(text, &span, scalar))
        })
        .count();
    let mut kinds = vec![CandidateType::Text];
    if fields
        .iter()
        .any(|field| field.candidate_type == CandidateType::PersonName)
        && name_allowed(text, &span, scalar)
    {
        kinds.push(CandidateType::PersonName);
    }
    for kind in kinds {
        hypotheses.push(Hypothesis {
            index: candidates.len(),
            owner: None,
        });
        candidates.push(candidate(
            text,
            source,
            span.clone(),
            kind,
            false,
            "residual_text",
        ));
    }
    eligible > 1
}

pub(super) fn complete(
    text: &str,
    candidates: &mut Vec<FieldCandidate>,
    fields: &[AssignmentField],
    header: Option<&TableHeaderContext>,
    sources: &[TextSource],
    result: &mut AssignmentResult,
    mut prepare_candidate: Option<&mut dyn FnMut(&mut FieldCandidate)>,
) {
    if !fields.iter().any(|field| is_text(&field.candidate_type)) {
        return;
    }
    let scalar = candidates.clone();
    let mut hypotheses = Vec::new();
    for source in sources {
        let (regions, syntax) = regions(text, source, fields, header);
        for region in &regions {
            let Some(span) = trim_span(text, &region.span) else {
                continue;
            };
            let whole = candidate(
                text,
                source,
                span.clone(),
                CandidateType::Text,
                true,
                region.reason,
            );
            let occupied: Vec<_> = result
                .fields
                .iter()
                .flat_map(|field| &field.candidates)
                .filter(|prior| overlaps(&whole, prior))
                .map(|prior| prior.source_span.clone())
                .collect();
            if !occupied.is_empty() {
                warning(result, "text_evidence_overlap");
                for gap in gaps(&span, occupied.into_iter()) {
                    if residual(
                        text,
                        source,
                        gap,
                        fields,
                        &scalar,
                        candidates,
                        &mut hypotheses,
                    ) {
                        warning(result, "text_field_ambiguous");
                    }
                }
                continue;
            }
            let owners: Vec<_> = region
                .fields
                .iter()
                .copied()
                .filter(|index| {
                    fields[*index].candidate_type == CandidateType::Text
                        || name_allowed(text, &span, &scalar)
                })
                .collect();
            let ambiguous = region.ambiguous || owners.len() > 1;
            if ambiguous {
                warning(result, "text_field_ambiguous");
            }
            let mut kinds = Vec::new();
            for index in &owners {
                if !kinds.contains(&fields[*index].candidate_type) {
                    kinds.push(fields[*index].candidate_type.clone());
                }
            }
            for kind in kinds {
                hypotheses.push(Hypothesis {
                    index: candidates.len(),
                    owner: (!ambiguous).then(|| owners[0]),
                });
                candidates.push(candidate(
                    text,
                    source,
                    span.clone(),
                    kind,
                    true,
                    region.reason,
                ));
            }
        }
        let excluded = scalar
            .iter()
            .map(|c| c.source_span.clone())
            .chain(syntax)
            .chain(regions.iter().map(|region| region.span.clone()));
        for gap in gaps(&source.span, excluded) {
            if residual(
                text,
                source,
                gap,
                fields,
                &scalar,
                candidates,
                &mut hypotheses,
            ) {
                warning(result, "text_field_ambiguous");
            }
        }
    }
    if let Some(prepare) = &mut prepare_candidate {
        for index in hypotheses.iter().map(|hypothesis| hypothesis.index) {
            prepare(&mut candidates[index]);
        }
    }
    select(candidates, fields, hypotheses, result);
}

fn select(
    candidates: &[FieldCandidate],
    fields: &[AssignmentField],
    mut hypotheses: Vec<Hypothesis>,
    result: &mut AssignmentResult,
) {
    // Resolve all interval conflicts before selecting any field, so schema order
    // cannot decide ownership. Constraints cannot resolve competing hypotheses.
    let conflicted: Vec<_> = hypotheses
        .iter()
        .enumerate()
        .filter(|(i, a)| {
            a.owner.is_some()
                && hypotheses.iter().enumerate().any(|(j, b)| {
                    i != &j
                        && b.owner.is_some()
                        && overlaps(&candidates[a.index], &candidates[b.index])
                })
        })
        .map(|(i, _)| i)
        .collect();
    if !conflicted.is_empty() {
        warning(result, "text_field_ambiguous");
    }
    for i in conflicted {
        hypotheses[i].owner = None;
    }
    let mut selected = Vec::new();
    for (index, field) in fields
        .iter()
        .enumerate()
        .filter(|(_, field)| is_text(&field.candidate_type))
    {
        let mut eligible: Vec<_> = hypotheses
            .iter()
            .filter(|hypothesis| hypothesis.owner == Some(index))
            .map(|hypothesis| hypothesis.index)
            .collect();
        eligible.sort_by_key(|i| {
            (
                candidates[*i].source_span.byte_start,
                candidates[*i].source_span.byte_end,
            )
        });
        if !field.multiple && eligible.len() > 1 {
            warning(result, "text_field_ambiguous");
            eligible.clear();
        }
        eligible.retain(|i| candidate_satisfies_constraints(&candidates[*i], field));
        if field.unique && eligible.len() > 1 {
            result.warnings.push(ParserWarning {
                code: "unique_field_multiple_values".into(),
                message: format!("unique field {} received multiple candidates", field.name),
                location: None,
            });
        }
        if eligible.is_empty() {
            if field.required {
                result.warnings.push(ParserWarning {
                    code: "required_field_missing".into(),
                    message: format!("required field {} has no compatible candidate", field.name),
                    location: None,
                });
            }
        } else {
            result.fields.push(AssignedField {
                name: field.name.clone(),
                candidates: eligible.iter().map(|i| candidates[*i].clone()).collect(),
            });
            selected.extend(eligible);
        }
    }
    result.unassigned_candidates.extend(
        hypotheses
            .into_iter()
            .filter(|hypothesis| !selected.contains(&hypothesis.index))
            .map(|hypothesis| candidates[hypothesis.index].clone()),
    );
    result.fields.sort_by_key(|assigned| {
        fields
            .iter()
            .position(|field| field.name == assigned.name)
            .unwrap_or(usize::MAX)
    });
}

/// Supplied candidates use the same ownership/selection rules, but this lower
/// level API never invents additional candidates or trusts caller reason strings.
pub(super) fn assign_supplied(
    text: &str,
    candidates: &[FieldCandidate],
    fields: &[AssignmentField],
    header: Option<&TableHeaderContext>,
    result: &mut AssignmentResult,
) {
    if !fields.iter().any(|field| is_text(&field.candidate_type)) {
        return;
    }
    let source = TextSource {
        span: TextSpan {
            byte_start: 0,
            byte_end: text.len(),
        },
        reference: None,
        column: None,
    };
    let (regions, _) = regions(text, &source, fields, None);
    let mut hypotheses = Vec::new();
    for (index, candidate) in candidates
        .iter()
        .enumerate()
        .filter(|(_, candidate)| is_text(&candidate.candidate_type))
    {
        let mut owners: Vec<_> = fields
            .iter()
            .enumerate()
            .filter(|(_, field)| {
                is_text(&field.candidate_type)
                    && candidate_matches_field_header(candidate, field, header)
            })
            .map(|(i, _)| i)
            .collect();
        let mut ambiguous = false;
        if owners.is_empty() {
            for region in &regions {
                if trim_span(text, &region.span).as_ref() == Some(&candidate.source_span) {
                    owners.extend(region.fields.iter().copied());
                    ambiguous |= region.ambiguous;
                }
            }
        }
        owners.retain(|i| {
            fields[*i].candidate_type == CandidateType::Text
                || name_allowed(text, &candidate.source_span, candidates)
        });
        if owners.len() > 1 || ambiguous {
            warning(result, "text_field_ambiguous");
            owners.clear();
        }
        owners.retain(|i| fields[*i].candidate_type == candidate.candidate_type);
        if result
            .fields
            .iter()
            .flat_map(|field| &field.candidates)
            .any(|prior| overlaps(candidate, prior))
        {
            warning(result, "text_evidence_overlap");
            owners.clear();
        }
        hypotheses.push(Hypothesis {
            index,
            owner: owners.first().copied(),
        });
    }
    result
        .unassigned_candidates
        .retain(|candidate| !is_text(&candidate.candidate_type));
    select(candidates, fields, hypotheses, result);
}

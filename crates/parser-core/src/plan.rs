use super::*;

pub(super) type EnumDefinitions = Vec<(String, Vec<String>)>;

/// One runtime field and its own enum vocabulary. This is not a wire model.
#[derive(Debug, Clone)]
pub struct PlanField {
    assignment: AssignmentField,
    enum_definitions: EnumDefinitions,
}

impl PlanField {
    /// Low-level runtime construction; use parser-schema to validate schema capabilities.
    pub fn new(assignment: AssignmentField, enum_definitions: Vec<(String, Vec<String>)>) -> Self {
        Self {
            assignment,
            enum_definitions,
        }
    }
}

/// Immutable runtime instructions compiled by parser-schema. No input is stored.
#[derive(Debug, Clone)]
pub struct ParsePlan {
    fields: Vec<AssignmentField>,
    enum_definitions: Vec<EnumDefinitions>,
    record_name: Option<String>,
    text_pipeline: Option<TextPipelineOptions>,
}

impl ParsePlan {
    /// Assemble runtime instructions without adding a serialized schema contract.
    pub fn new(fields: Vec<PlanField>, record_name: Option<String>) -> Self {
        let (fields, enum_definitions) = fields
            .into_iter()
            .map(|field| (field.assignment, field.enum_definitions))
            .unzip();
        Self {
            fields,
            enum_definitions,
            record_name,
            text_pipeline: None,
        }
    }

    pub(super) fn fields(&self) -> &[AssignmentField] {
        &self.fields
    }

    pub(super) fn enum_definitions(&self) -> &[EnumDefinitions] {
        &self.enum_definitions
    }

    pub(super) fn record_name(&self) -> Option<String> {
        self.record_name.clone()
    }

    pub(super) fn text_pipeline_enabled(&self) -> bool {
        self.text_pipeline.is_some()
    }

    /// Parser-schema's deliberate bridge for validated runtime options.
    #[doc(hidden)]
    pub fn with_text_pipeline(mut self, options: TextPipelineOptions) -> Self {
        self.text_pipeline = Some(options);
        self
    }
}

/// Parse a canonical document with field-scoped enum instructions and full evidence.
pub fn parse_document_with_plan(document: &RawDocument, plan: &ParsePlan) -> ParseResponse {
    parse_document_inner(
        document,
        &plan.fields,
        DetectionRules::Scoped(&plan.enum_definitions),
        plan.record_name.clone(),
        plan.text_pipeline.as_ref(),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseLimits {
    pub max_records: usize,
    pub max_response_bytes: usize,
}
impl Default for ParseLimits {
    fn default() -> Self {
        Self {
            max_records: 100_000,
            max_response_bytes: 16 * 1024 * 1024,
        }
    }
}

pub fn parse_document_with_plan_with_limits(
    document: &RawDocument,
    plan: &ParsePlan,
    limits: ParseLimits,
) -> Result<ParseResponse, Failure> {
    let response = parse_document_with_plan(document, plan);
    enforce_parse_response_limits(&response, limits)?;
    Ok(response)
}

/// Shared post-parse limit enforcement for core and formats-owned orchestration.
///
/// The response already exists when these two derived properties become known;
/// serialization is counted without allocating a second response buffer.
#[doc(hidden)]
pub fn enforce_parse_response_limits(
    response: &ParseResponse,
    limits: ParseLimits,
) -> Result<(), Failure> {
    let records = match &response.content {
        ParseContent::Text { records } => records.len(),
        ParseContent::Table { sheets } => sheets.iter().fold(0usize, |count, sheet| {
            count.saturating_add(sheet.records.len())
        }),
    };
    if records > limits.max_records {
        return Err(Failure::new(FailureKind::ResourceLimit {
            resource: ResourceLimitKind::Records,
            limit: limits.max_records as u64,
            actual: records as u64,
        }));
    }

    let mut counter = CountingWriter::default();
    serde_json::to_writer(&mut counter, response).map_err(|_| {
        Failure::new(FailureKind::OutputSerialization {
            target: OutputTarget::ParseResult,
        })
    })?;
    if counter.bytes > limits.max_response_bytes {
        return Err(Failure::new(FailureKind::ResourceLimit {
            resource: ResourceLimitKind::ResponseBytes,
            limit: limits.max_response_bytes as u64,
            actual: counter.bytes as u64,
        }));
    }
    Ok(())
}

#[derive(Default)]
struct CountingWriter {
    bytes: usize,
}

impl std::io::Write for CountingWriter {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        self.bytes = self
            .bytes
            .checked_add(buffer.len())
            .ok_or_else(|| std::io::Error::other("serialized response size overflow"))?;
        Ok(buffer.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[derive(Clone, Copy)]
pub(super) enum DetectionRules<'a> {
    Legacy(&'a [(String, Vec<String>)]),
    Scoped(&'a [EnumDefinitions]),
}

impl<'a> DetectionRules<'a> {
    pub(super) fn scoped(self) -> Option<&'a [EnumDefinitions]> {
        match self {
            Self::Legacy(_) => None,
            Self::Scoped(definitions) => Some(definitions),
        }
    }

    pub(super) fn detect_enums(self, text: &str) -> Vec<FieldCandidate> {
        match self {
            Self::Legacy(definitions) => detect_enum_candidates(text, definitions),
            Self::Scoped(fields) => {
                let mut candidates = Vec::new();
                for definitions in fields {
                    for candidate in detect_enum_candidates(text, definitions) {
                        if !candidates.contains(&candidate) {
                            candidates.push(candidate);
                        }
                    }
                }
                // Stable occurrence order; canonical alternatives retain field order.
                candidates.sort_by_key(|candidate| {
                    (
                        candidate.source_span.byte_start,
                        candidate.source_span.byte_end,
                    )
                });
                candidates
            }
        }
    }
}

/// Resolve enum ownership before greedy field assignment. Constraints cannot
/// manufacture ownership, and tied context never falls back to schema order.
pub(super) fn enum_ownership(
    text: &str,
    candidates: &[FieldCandidate],
    fields: &[AssignmentField],
    header: Option<&TableHeaderContext>,
    definitions: Option<&[EnumDefinitions]>,
    context_spans: Option<&[TextSpan]>,
) -> (Option<Vec<Option<usize>>>, Vec<ParserWarning>) {
    let Some(definitions) = definitions else {
        return (None, Vec::new());
    };
    let mut owners = vec![None; candidates.len()];
    let mut warnings = Vec::new();
    let mut seen = Vec::new();
    for candidate in candidates
        .iter()
        .filter(|candidate| candidate.candidate_type == CandidateType::Enum)
    {
        if seen.contains(&candidate.source_span) {
            continue;
        }
        seen.push(candidate.source_span.clone());
        let mut matches = Vec::new();
        for (field_index, field) in fields.iter().enumerate() {
            if field.candidate_type != CandidateType::Enum {
                continue;
            }
            for (index, alternative) in candidates.iter().enumerate().filter(|(_, alternative)| {
                alternative.candidate_type == CandidateType::Enum
                    && alternative.source_span == candidate.source_span
            }) {
                let lexical_match = definitions[field_index].iter().any(|(canonical, aliases)| {
                    alternative
                        .normalized_value
                        .as_ref()
                        .and_then(serde_json::Value::as_str)
                        == Some(canonical.as_str())
                        && (canonical.eq_ignore_ascii_case(&alternative.raw_value)
                            || aliases
                                .iter()
                                .any(|alias| alias.eq_ignore_ascii_case(&alternative.raw_value)))
                });
                if lexical_match {
                    let score = candidate_score(text, alternative, field, header, context_spans);
                    matches.push((field_index, index, (score.0, score.1)));
                }
            }
        }
        let best = matches.iter().map(|(_, _, score)| *score).max();
        let best_matches: Vec<_> = matches
            .iter()
            .filter(|(_, _, score)| Some(*score) == best)
            .collect();
        if let [(field_index, candidate_index, _)] = best_matches.as_slice() {
            owners[*candidate_index] = Some(*field_index);
        } else if !matches.is_empty() {
            warnings.push(ParserWarning {
                code: "enum_field_ambiguous".to_owned(),
                message: "an enum occurrence matches multiple fields without uniquely decisive header or label context".to_owned(),
                location: None,
            });
        }
    }
    (Some(owners), warnings)
}

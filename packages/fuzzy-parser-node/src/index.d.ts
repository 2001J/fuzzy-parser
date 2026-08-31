export type InputFormat = "text" | "txt" | "csv" | "xlsx";

export interface ParserInput {
  format: InputFormat;
  bytes: ArrayBuffer | ArrayBufferView;
  /** Safe basename metadata only. Paths, NUL bytes, and values over 255 UTF-8 bytes are rejected. */
  filename?: string;
}

export interface TargetSchema {
  schema_version: "0.1";
  record_name: string | null;
  fields: ReadonlyArray<Record<string, unknown>>;
  options: {
    allow_unknown_fields: boolean;
    text_pipeline?: Record<string, unknown>;
  };
}

export type ProfileScalarFieldType =
  | "text"
  | "person_name"
  | "phone_number"
  | "email"
  | "integer"
  | "decimal"
  | "currency"
  | "date"
  | "datetime"
  | "boolean";

export interface ProfileEnumValue {
  value: string;
  aliases?: ReadonlyArray<string>;
}

export type ProfileFieldType =
  | ProfileScalarFieldType
  | { enum: { values: ReadonlyArray<ProfileEnumValue> } };

export type ProfileConstraint =
  | { kind: "minimumInteger"; value: number }
  | { kind: "maximumInteger"; value: number }
  | { kind: "minimumLength"; value: number }
  | { kind: "maximumLength"; value: number };

export interface ProfileTextPipeline {
  strategy: "one_block_per_record" | "join_indented_continuations" | "split_repeated_identifiers";
  repeatedIdentifierMarkers?: ReadonlyArray<string>;
  normalization?: {
    normalizeLineEndings?: boolean;
    trimWhitespace?: boolean;
    collapseWhitespace?: boolean;
    normalizePunctuation?: boolean;
    markNoise?: boolean;
  };
}

export interface RowRange {
  start: number;
  end: number;
}

export type TableHeaderSelection =
  | { mode: "automatic" }
  | { mode: "none" }
  | { mode: "row"; row: number }
  | { mode: "schema_search"; maxRows: number };

export type TableSheetSelection =
  | { mode: "all" }
  | { mode: "selected"; sheets: ReadonlyArray<{ name: string } | { index: number }> };

export interface ParseOptions {
  tableSelection?: {
    header?: TableHeaderSelection;
    includeRows?: ReadonlyArray<RowRange>;
    excludeRows?: ReadonlyArray<RowRange>;
    sheets?: TableSheetSelection;
  };
}

export interface ParseRequest {
  input: ParserInput;
  /** The existing schema 0.1 object or its exact JSON representation. */
  schema: TargetSchema | string;
  options?: ParseOptions;
}

/** A field expressed once in application-owned vocabulary, not as parser JSON. */
export interface ProfileField {
  name: string;
  fieldType: ProfileFieldType;
  required?: boolean;
  multiple?: boolean;
  aliases?: ReadonlyArray<string>;
  constraints?: ReadonlyArray<ProfileConstraint>;
}

export interface ProfileDefinition {
  name: string;
  /** Caller-managed revision; separate from the engine schema version. */
  version: string;
  recordName?: string;
  fields: ReadonlyArray<ProfileField>;
  options?: {
    allowUnknownFields?: boolean;
    textPipeline?: ProfileTextPipeline;
  };
}

export interface ApplicationProfile {
  readonly name: string;
  readonly version: string;
  readonly schema: TargetSchema;
}

export interface ParseControls {
  timeoutMs?: number;
  signal?: AbortSignal;
}

export interface ParserReason {
  code: string;
  message: string;
}

export interface SourceSpan {
  byte_start: number;
  byte_end: number;
}

export interface SourceReference {
  block_index: number;
  coordinate_space: "raw_text_utf8" | "rendered_value_utf8";
  span: SourceSpan;
}

export interface ParserCandidate {
  candidate_type: string;
  raw_value: string;
  normalized_value: string | number | boolean | null;
  source_span: SourceSpan;
  source_column?: number | null;
  source_reference?: SourceReference | null;
  confidence: number;
  reasons: ReadonlyArray<ParserReason>;
}

export interface AssignedField {
  name: string;
  candidates: ReadonlyArray<ParserCandidate>;
}

export interface ParsedRecord {
  source_row?: number;
  source_block_id?: string;
  source_block_ids?: ReadonlyArray<string>;
  parse: {
    candidates: ReadonlyArray<ParserCandidate>;
    assignment: {
      fields: ReadonlyArray<AssignedField>;
      unassigned_candidates: ReadonlyArray<ParserCandidate>;
      warnings: ReadonlyArray<Record<string, unknown>>;
    };
    review: {
      status: "clear" | "needs_review";
      reasons: ReadonlyArray<ParserReason>;
    };
  };
  [key: string]: unknown;
}

export interface SourceEvidence {
  document: Record<string, unknown>;
  blocks: ReadonlyArray<Record<string, unknown>>;
  table?: Record<string, unknown>;
}

export type ParseContent =
  | { mode: "text"; records: ReadonlyArray<ParsedRecord> }
  | { mode: "table"; sheets: ReadonlyArray<{ records: ReadonlyArray<ParsedRecord>; [key: string]: unknown }> };

export interface ParseResponse {
  contract_version: "0.1";
  parser_version: "0.1.0";
  record_name: string | null;
  source_type: "text" | "stdin" | "txt" | "csv" | "xlsx";
  content: ParseContent;
  warnings: ReadonlyArray<Record<string, unknown>>;
  source_evidence?: SourceEvidence;
}

export interface UnresolvedRecordEvidence {
  record: ParsedRecord;
  candidates: ReadonlyArray<ParserCandidate>;
}

export interface UnresolvedEvidence {
  records: ReadonlyArray<UnresolvedRecordEvidence>;
  /** Canonical source and unused spans; null only for legacy responses without evidence. */
  source: SourceEvidence | null;
}

export interface ErrorReport {
  error: {
    error_contract_version: "0.1";
    code: string;
    [key: string]: unknown;
  };
  message: string;
}

export type AdapterErrorCode =
  | "INVALID_REQUEST"
  | "MESSAGE_LIMIT"
  | "ABORTED"
  | "TIMEOUT"
  | "INITIALIZATION_FAILED"
  | "PROTOCOL_ERROR"
  | "OUTPUT_LIMIT";

export class AdapterError extends Error {
  readonly name: "AdapterError";
  readonly code: AdapterErrorCode;
}

export class ParserFailure extends Error {
  readonly name: "ParserFailure";
  readonly code: string;
  readonly report: ErrorReport;
}

export const PACKAGE_LIMITS: Readonly<{
  maxMessageBytes: number;
  maxOptionsBytes: number;
  maxResultBytes: number;
  defaultTimeoutMs: number;
  maxTimeoutMs: number;
}>;

export function parse(request: ParseRequest, controls?: ParseControls): Promise<ParseResponse>;
/** Validates executable capabilities before input is supplied, then returns a reusable profile. */
export function defineProfile(definition: ProfileDefinition, controls?: ParseControls): Promise<ApplicationProfile>;
export function parseProfile(profile: ApplicationProfile, input: ParserInput, options?: ParseOptions, controls?: ParseControls): Promise<ParseResponse>;
export function records(response: ParseResponse): ReadonlyArray<ParsedRecord>;
export function reviewRecords(response: ParseResponse): ReadonlyArray<ParsedRecord>;
export function unresolvedEvidence(response: ParseResponse): UnresolvedEvidence;

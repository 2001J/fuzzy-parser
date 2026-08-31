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

export interface ParseControls {
  timeoutMs?: number;
  signal?: AbortSignal;
}

export interface ParseResponse {
  contract_version: "0.1";
  parser_version: "0.1.0";
  record_name: string | null;
  source_type: "text" | "stdin" | "txt" | "csv" | "xlsx";
  content: Record<string, unknown>;
  warnings: ReadonlyArray<Record<string, unknown>>;
  source_evidence?: Record<string, unknown>;
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

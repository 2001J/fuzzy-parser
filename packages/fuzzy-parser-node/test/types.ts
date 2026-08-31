import {
  AdapterError,
  defineProfile,
  parseProfile,
  records,
  reviewRecords,
  unresolvedEvidence,
  ParserFailure,
  parse,
  type ParseRequest,
  type ParseResponse,
} from "@fuzzy-parser/node";

const request: ParseRequest = {
  input: { format: "text", bytes: new TextEncoder().encode("Ada") },
  schema: {
    schema_version: "0.1",
    record_name: "person",
    fields: [],
    options: { allow_unknown_fields: true },
  },
};

const response: Promise<ParseResponse> = parse(request, {
  timeoutMs: 1_000,
  signal: new AbortController().signal,
});

void response;
void AdapterError;
void ParserFailure;

const profile = defineProfile({
  name: "contact",
  version: "1",
  fields: [{ name: "person", fieldType: "person_name", required: true }],
});
void profile.then(async (value) => {
  const parsed = await parseProfile(value, request.input);
  const rows = records(parsed);
  const review = reviewRecords(parsed);
  const evidence = unresolvedEvidence(parsed);
  void rows;
  void review;
  void evidence;
});

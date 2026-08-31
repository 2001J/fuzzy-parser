'use strict';

const runtime = require('./runtime.cjs');
const definedProfiles = new WeakSet();
const scalarFieldTypes = new Set([
  'text', 'person_name', 'phone_number', 'email', 'integer', 'decimal',
  'currency', 'date', 'datetime', 'boolean',
]);

function profileInvalid(message) {
  return new runtime.AdapterError('INVALID_REQUEST', message);
}

function deepFreeze(value) {
  if (value && typeof value === 'object' && !Object.isFrozen(value)) {
    for (const child of Object.values(value)) deepFreeze(child);
    Object.freeze(value);
  }
  return value;
}

function rejectUnknownKeys(value, allowed, label) {
  for (const key of Object.keys(value)) {
    if (!allowed.has(key)) throw profileInvalid(`${label}.${key} is not supported`);
  }
}

function normalizeFieldType(value) {
  if (typeof value === 'string') {
    if (!scalarFieldTypes.has(value)) throw profileInvalid('profile fieldType is not supported');
    return value;
  }
  if (!value || typeof value !== 'object' || !value.enum || typeof value.enum !== 'object') {
    throw profileInvalid('profile fieldType must be a supported type or enum definition');
  }
  rejectUnknownKeys(value, new Set(['enum']), 'profile fieldType');
  rejectUnknownKeys(value.enum, new Set(['values']), 'profile enum');
  if (!Array.isArray(value.enum.values)) throw profileInvalid('profile enum values must be an array');
  return {
    enum: {
      values: value.enum.values.map((entry) => {
        if (!entry || typeof entry !== 'object') throw profileInvalid('profile enum value must be an object');
        rejectUnknownKeys(entry, new Set(['value', 'aliases']), 'profile enum value');
        if (typeof entry.value !== 'string') throw profileInvalid('profile enum value.value must be a string');
        if (entry.aliases !== undefined && (!Array.isArray(entry.aliases) || entry.aliases.some((alias) => typeof alias !== 'string'))) {
          throw profileInvalid('profile enum value.aliases must be strings');
        }
        return { value: entry.value, aliases: entry.aliases ?? [] };
      }),
    },
  };
}

function normalizeConstraints(values) {
  if (values === undefined) return [];
  if (!Array.isArray(values)) throw profileInvalid('profile field constraints must be an array');
  const kinds = {
    minimumInteger: 'minimum_integer',
    maximumInteger: 'maximum_integer',
    minimumLength: 'minimum_length',
    maximumLength: 'maximum_length',
  };
  return values.map((constraint) => {
    if (!constraint || typeof constraint !== 'object') throw profileInvalid('profile constraint must be an object');
    rejectUnknownKeys(constraint, new Set(['kind', 'value']), 'profile constraint');
    const kind = kinds[constraint.kind];
    if (!kind || !Number.isSafeInteger(constraint.value)) throw profileInvalid('profile constraint kind/value is invalid');
    if ((kind === 'minimum_length' || kind === 'maximum_length') && constraint.value < 0) {
      throw profileInvalid('profile length constraint must not be negative');
    }
    return { kind, value: constraint.value };
  });
}

function normalizeTextPipeline(value) {
  if (!value || typeof value !== 'object') throw profileInvalid('profile.options.textPipeline must be an object');
  rejectUnknownKeys(value, new Set(['strategy', 'repeatedIdentifierMarkers', 'normalization']), 'profile.options.textPipeline');
  const strategies = new Set(['one_block_per_record', 'join_indented_continuations', 'split_repeated_identifiers']);
  if (!strategies.has(value.strategy)) throw profileInvalid('profile text pipeline strategy is invalid');
  const markers = value.repeatedIdentifierMarkers ?? [];
  if (!Array.isArray(markers) || markers.some((marker) => typeof marker !== 'string')) {
    throw profileInvalid('profile text pipeline markers must be strings');
  }
  const normalization = value.normalization ?? {};
  if (!normalization || typeof normalization !== 'object') throw profileInvalid('profile text normalization must be an object');
  rejectUnknownKeys(normalization, new Set([
    'normalizeLineEndings', 'trimWhitespace', 'collapseWhitespace', 'normalizePunctuation', 'markNoise',
  ]), 'profile text normalization');
  for (const setting of Object.values(normalization)) {
    if (typeof setting !== 'boolean') throw profileInvalid('profile text normalization settings must be booleans');
  }
  return {
    normalization: {
      normalize_line_endings: normalization.normalizeLineEndings ?? true,
      trim_whitespace: normalization.trimWhitespace ?? true,
      collapse_whitespace: normalization.collapseWhitespace ?? true,
      normalize_punctuation: normalization.normalizePunctuation ?? true,
      mark_noise: normalization.markNoise ?? true,
    },
    strategy: value.strategy,
    repeated_identifier_markers: markers,
  };
}

function normalizeProfile(definition) {
  if (!definition || typeof definition !== 'object') throw profileInvalid('profile definition is required');
  rejectUnknownKeys(definition, new Set(['name', 'version', 'recordName', 'fields', 'options']), 'profile');
  const { name, version, recordName = name, fields, options = {} } = definition;
  if (typeof name !== 'string' || name.trim().length === 0) throw profileInvalid('profile.name must be a non-empty string');
  if (typeof version !== 'string' || version.trim().length === 0) throw profileInvalid('profile.version must be a non-empty string');
  if (typeof recordName !== 'string' || recordName.trim().length === 0) throw profileInvalid('profile.recordName must be a non-empty string');
  if (!Array.isArray(fields)) throw profileInvalid('profile.fields must be an array');
  if (!options || typeof options !== 'object') throw profileInvalid('profile.options must be an object');
  rejectUnknownKeys(options, new Set(['allowUnknownFields', 'textPipeline']), 'profile.options');
  if (options.allowUnknownFields !== undefined && typeof options.allowUnknownFields !== 'boolean') {
    throw profileInvalid('profile.options.allowUnknownFields must be a boolean');
  }
  const schema = {
    schema_version: '0.1',
    record_name: recordName,
    fields: fields.map((field) => {
      if (!field || typeof field !== 'object' || typeof field.name !== 'string' || !('fieldType' in field)) {
        throw profileInvalid('each profile field requires name and fieldType');
      }
      rejectUnknownKeys(field, new Set(['name', 'fieldType', 'required', 'multiple', 'aliases', 'constraints']), 'profile field');
      if (field.required !== undefined && typeof field.required !== 'boolean') throw profileInvalid('profile field.required must be a boolean');
      if (field.multiple !== undefined && typeof field.multiple !== 'boolean') throw profileInvalid('profile field.multiple must be a boolean');
      if (field.aliases !== undefined && (!Array.isArray(field.aliases) || field.aliases.some((alias) => typeof alias !== 'string'))) {
        throw profileInvalid('profile field.aliases must be strings');
      }
      return {
        name: field.name,
        field_type: normalizeFieldType(field.fieldType),
        required: field.required ?? false,
        multiple: field.multiple ?? false,
        aliases: field.aliases ?? [],
        constraints: normalizeConstraints(field.constraints),
      };
    }),
    options: {
      allow_unknown_fields: options.allowUnknownFields ?? true,
      ...(options.textPipeline === undefined ? {} : { text_pipeline: normalizeTextPipeline(options.textPipeline) }),
    },
  };
  return deepFreeze({ name, version, schema });
}

/** Validates a profile through the same Worker/WASM compiler before input arrives. */
async function defineProfile(definition, controls) {
  const profile = normalizeProfile(definition);
  await runtime.parse({
    input: { format: 'text', bytes: new Uint8Array() },
    schema: profile.schema,
  }, controls);
  definedProfiles.add(profile);
  return profile;
}

function parseProfile(profile, input, options, controls) {
  if (!profile || typeof profile !== 'object' || !definedProfiles.has(profile)) {
    throw profileInvalid('profile must be returned by defineProfile');
  }
  return runtime.parse({ input, schema: profile.schema, ...(options === undefined ? {} : { options }) }, controls);
}

function records(response) {
  if (!response || typeof response !== 'object') throw profileInvalid('response is required');
  if (Array.isArray(response.content?.records)) return response.content.records;
  if (Array.isArray(response.content?.sheets)) return response.content.sheets.flatMap((sheet) => sheet.records ?? []);
  return [];
}

function reviewRecords(response) {
  return records(response).filter((record) => record?.parse?.review?.status === 'needs_review');
}

function unresolvedEvidence(response) {
  return {
    records: records(response)
      .map((record) => ({
        record,
        candidates: record?.parse?.assignment?.unassigned_candidates ?? [],
      }))
      .filter((entry) => entry.candidates.length > 0),
    source: response?.source_evidence ?? null,
  };
}

exports.parse = runtime.parse;
exports.AdapterError = runtime.AdapterError;
exports.ParserFailure = runtime.ParserFailure;
exports.PACKAGE_LIMITS = runtime.PACKAGE_LIMITS;
exports.defineProfile = defineProfile;
exports.parseProfile = parseProfile;
exports.records = records;
exports.reviewRecords = reviewRecords;
exports.unresolvedEvidence = unresolvedEvidence;

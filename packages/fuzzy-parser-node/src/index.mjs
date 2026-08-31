import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
// The CJS entry owns the small profile helpers as well as the Worker runtime.
const runtime = require('./index.cjs');

export const parse = runtime.parse;
export const AdapterError = runtime.AdapterError;
export const ParserFailure = runtime.ParserFailure;
export const PACKAGE_LIMITS = runtime.PACKAGE_LIMITS;
export const defineProfile = runtime.defineProfile;
export const parseProfile = runtime.parseProfile;
export const records = runtime.records;
export const reviewRecords = runtime.reviewRecords;
export const unresolvedEvidence = runtime.unresolvedEvidence;

import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const runtime = require('./runtime.cjs');

export const parse = runtime.parse;
export const AdapterError = runtime.AdapterError;
export const ParserFailure = runtime.ParserFailure;
export const PACKAGE_LIMITS = runtime.PACKAGE_LIMITS;

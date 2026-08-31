'use strict';

const runtime = require('./runtime.cjs');

exports.parse = runtime.parse;
exports.AdapterError = runtime.AdapterError;
exports.ParserFailure = runtime.ParserFailure;
exports.PACKAGE_LIMITS = runtime.PACKAGE_LIMITS;

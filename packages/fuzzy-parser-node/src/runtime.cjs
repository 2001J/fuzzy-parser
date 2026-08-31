'use strict';

const { Worker } = require('node:worker_threads');

const WORKER_PATH = require.resolve('./worker.cjs');
const PROTOCOL_VERSION = 1;
const PACKAGE_LIMITS = Object.freeze({
  maxMessageBytes: 70 * 1024 * 1024,
  maxOptionsBytes: 64 * 1024,
  maxResultBytes: 16 * 1024 * 1024,
  defaultTimeoutMs: 30_000,
  maxTimeoutMs: 120_000,
});
const FORMATS = new Set(['text', 'txt', 'csv', 'xlsx']);
const activeWorkers = new Set();
let nextRequestId = 1;
let workerObserver;

class AdapterError extends Error {
  constructor(code, message) {
    super(message);
    this.name = 'AdapterError';
    this.code = code;
  }
}

class ParserFailure extends Error {
  constructor(report) {
    super(report.message);
    this.name = 'ParserFailure';
    this.code = report.error.code;
    this.report = report;
  }
}

function invalid(message) {
  return new AdapterError('INVALID_REQUEST', message);
}

function byteLength(value) {
  return Buffer.byteLength(value, 'utf8');
}

function normalizeBytes(value) {
  if (value instanceof ArrayBuffer) return new Uint8Array(value.slice(0));
  if (ArrayBuffer.isView(value)) {
    return new Uint8Array(value.buffer.slice(value.byteOffset, value.byteOffset + value.byteLength));
  }
  throw invalid('input.bytes must be an ArrayBuffer or an ArrayBuffer view');
}

function normalizeFilename(filename) {
  if (filename === undefined) return undefined;
  if (typeof filename !== 'string' || filename.length === 0) {
    throw invalid('input.filename must be a non-empty string when provided');
  }
  if (filename.includes('\0') || filename.includes('/') || filename.includes('\\')) {
    throw invalid('input.filename must be basename metadata, not a path');
  }
  if (byteLength(filename) > 255) throw invalid('input.filename exceeds 255 UTF-8 bytes');
  return filename;
}

function encodeJson(value, label) {
  if (typeof value === 'string') return value;
  try {
    const encoded = JSON.stringify(value);
    if (encoded === undefined) throw new TypeError('not JSON serializable');
    return encoded;
  } catch {
    throw invalid(`${label} must be JSON serializable`);
  }
}

function normalizeRequest(request) {
  if (!request || typeof request !== 'object' || !request.input || typeof request.input !== 'object') {
    throw invalid('request.input is required');
  }
  const { format } = request.input;
  if (!FORMATS.has(format)) throw invalid('input.format must be text, txt, csv, or xlsx');
  const bytes = normalizeBytes(request.input.bytes);
  const filename = normalizeFilename(request.input.filename);
  const schema = encodeJson(request.schema, 'schema');
  const options = request.options === undefined ? undefined : encodeJson(request.options, 'options');
  if (options !== undefined && byteLength(options) > PACKAGE_LIMITS.maxOptionsBytes) {
    throw new AdapterError('MESSAGE_LIMIT', 'options exceed the package message limit');
  }
  const messageBytes = bytes.byteLength + byteLength(schema) + byteLength(filename ?? '') + byteLength(options ?? '');
  if (messageBytes > PACKAGE_LIMITS.maxMessageBytes) {
    throw new AdapterError('MESSAGE_LIMIT', 'request exceeds the package message limit');
  }
  return { format, bytes, filename, schema, options };
}

function normalizeControls(controls) {
  const timeoutMs = controls?.timeoutMs ?? PACKAGE_LIMITS.defaultTimeoutMs;
  if (!Number.isFinite(timeoutMs) || timeoutMs <= 0 || timeoutMs > PACKAGE_LIMITS.maxTimeoutMs) {
    throw invalid(`controls.timeoutMs must be between 1 and ${PACKAGE_LIMITS.maxTimeoutMs}`);
  }
  const signal = controls?.signal;
  if (signal !== undefined && !(signal instanceof AbortSignal)) {
    throw invalid('controls.signal must be an AbortSignal');
  }
  return { timeoutMs, signal };
}

function decodeReport(json) {
  if (byteLength(json) > PACKAGE_LIMITS.maxResultBytes) {
    throw new AdapterError('OUTPUT_LIMIT', 'parser result exceeds the package output limit');
  }
  let report;
  try {
    report = JSON.parse(json);
  } catch {
    throw new AdapterError('PROTOCOL_ERROR', 'worker returned invalid parser failure JSON');
  }
  if (!report?.error || report.error.error_contract_version !== '0.1' || typeof report.error.code !== 'string' || typeof report.message !== 'string') {
    throw new AdapterError('PROTOCOL_ERROR', 'worker returned an incompatible parser failure');
  }
  return report;
}

function decodeSuccess(json) {
  if (byteLength(json) > PACKAGE_LIMITS.maxResultBytes) {
    throw new AdapterError('OUTPUT_LIMIT', 'parser result exceeds the package output limit');
  }
  let response;
  try {
    response = JSON.parse(json);
  } catch {
    throw new AdapterError('PROTOCOL_ERROR', 'worker returned invalid parser result JSON');
  }
  if (response?.contract_version !== '0.1' || response?.parser_version !== '0.1.0') {
    throw new AdapterError('PROTOCOL_ERROR', 'worker returned an incompatible parser result');
  }
  return response;
}

async function parse(request, controls) {
  const normalized = normalizeRequest(request);
  const { timeoutMs, signal } = normalizeControls(controls);
  if (signal?.aborted) throw new AdapterError('ABORTED', 'parse was aborted');

  return new Promise((resolve, reject) => {
    const requestId = nextRequestId++;
    const worker = new Worker(WORKER_PATH);
    activeWorkers.add(worker);
    let settled = false;
    let ready = false;

    const cleanup = () => {
      clearTimeout(timer);
      signal?.removeEventListener('abort', onAbort);
      activeWorkers.delete(worker);
    };
    const finish = async (error, value) => {
      if (settled) return;
      settled = true;
      try {
        await worker.terminate();
      } catch {
        if (!error) {
          error = new AdapterError('INITIALIZATION_FAILED', 'parser Worker cleanup failed');
        }
      } finally {
        cleanup();
      }
      if (error) reject(error);
      else resolve(value);
    };
    const onAbort = () => void finish(new AdapterError('ABORTED', 'parse was aborted'));
    const timer = setTimeout(
      () => void finish(new AdapterError('TIMEOUT', `parse exceeded ${timeoutMs} ms`)),
      timeoutMs,
    );
    signal?.addEventListener('abort', onAbort, { once: true });

    worker.on('message', (message) => {
      workerObserver?.(message);
      if (settled) return;
      if (message?.type === 'entered' && message.requestId === requestId) return;
      if (message?.type === 'ready' && message.protocolVersion === PROTOCOL_VERSION && !ready) {
        ready = true;
        worker.postMessage(
          { type: 'parse', protocolVersion: PROTOCOL_VERSION, requestId, ...normalized },
          [normalized.bytes.buffer],
        );
        return;
      }
      if (message?.type === 'adapter_error') {
        void finish(new AdapterError(message.code ?? 'PROTOCOL_ERROR', message.message ?? 'worker failed'));
        return;
      }
      if (message?.type !== 'result' || message.requestId !== requestId || typeof message.json !== 'string') {
        void finish(new AdapterError('PROTOCOL_ERROR', 'worker returned a malformed message'));
        return;
      }
      try {
        if (message.outcome === 'success') void finish(undefined, decodeSuccess(message.json));
        else if (message.outcome === 'parser_failure') void finish(new ParserFailure(decodeReport(message.json)));
        else void finish(new AdapterError('PROTOCOL_ERROR', 'worker returned an unknown outcome'));
      } catch (error) {
        void finish(error);
      }
    });
    worker.once('error', () => void finish(new AdapterError('INITIALIZATION_FAILED', 'parser Worker failed')));
    worker.once('exit', () => {
      if (!settled) void finish(new AdapterError('INITIALIZATION_FAILED', 'parser Worker exited before returning a result'));
    });
  });
}

const __testing = Object.freeze({
  activeWorkerCount: () => activeWorkers.size,
  setWorkerObserver(observer) {
    workerObserver = observer;
  },
});

module.exports = { parse, AdapterError, ParserFailure, PACKAGE_LIMITS, __testing };

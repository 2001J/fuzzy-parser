# 2026-08-30 — Node/WASM Runtime Evaluation

> **Historical evaluation.** Current integration guidance lives in the
> [Integration guide](../integration-strategy.md). Measurements and conclusions
> here describe the dated experiment only.

## Scope

This is an evaluation-only `wasm32-unknown-unknown` cdylib in
[`evaluations/wasm-runtime`](../../evaluations/wasm-runtime). Its manifest and
lock file are outside the Cargo workspace. It is not a package, a public API,
or a second adapter.

The boundary accepts declared `text`, `txt`, `csv` or `xlsx` bytes, an optional
filename, and schema JSON, then returns the existing pretty JSON success or
safe-error string. It calls, in order, strict schema decoding, byte extraction,
schema compilation, and `parse_document_with_plan`. The native oracle is
compiled from the same `00a15fa` baseline and shares this exact function; it is
not a comparison with a prior CLI binary.

## Local evidence

Environment: macOS arm64, Node `v22.14.0`, Rust `1.96.0`, and task-local
`wasm-bindgen-cli 0.2.115`. Reproduce the task-local tool acquisition with:

```text
cargo install wasm-bindgen-cli --version 0.2.115 --locked --root evaluations/wasm-runtime/.toolchain
```

The isolated lock pins the matching
`wasm-bindgen = 0.2.115`; no global installation, npm install, publication, or
host integration was performed.

Commands run:

```text
cargo test --locked --manifest-path evaluations/wasm-runtime/Cargo.toml
cargo build --locked --manifest-path evaluations/wasm-runtime/Cargo.toml --target wasm32-unknown-unknown --release
evaluations/wasm-runtime/.toolchain/bin/wasm-bindgen --target nodejs --out-dir evaluations/wasm-runtime/pkg evaluations/wasm-runtime/target/wasm32-unknown-unknown/release/fuzzy_parser_wasm_evaluation.wasm
node --check evaluations/wasm-runtime/harness.mjs
node --check evaluations/wasm-runtime/worker.cjs
node evaluations/wasm-runtime/harness.mjs
```

The harness made 58 exact WASM/native byte-path comparisons (29 through each
CJS and ESM loader): eight profile and
format successes for two unrelated synthetic profiles, optional-filename TXT,
CSV and XLSX calls, Unicode/blank TXT and typed/unicode XLSX calls, 10/100/500
CSV rows, text/person-name controls, nine expected safe errors, and the
large-integer provenance case. It resolves every detected, assigned, and
unassigned candidate reference to a source block and checks ordered UTF-8 byte
spans. The QualEvents-shaped profile is synthetic fixture data only; no
QualEvents checkout or dependency was used.

The nine safe error payloads match native exactly: malformed schema, schema
version, unsupported datetime, invalid enum definition, invalid UTF-8, CSV,
XLSX, line length, and unknown format. Text and person-name controls remain
successful, reflecting current engine behavior. The source evidence retains
`9007199254740993` as a string, avoiding a JavaScript numeric conversion for
provenance.

Generated Node glue and WASM artifact measurements from the final run:

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| `fuzzy_parser_wasm_evaluation.js` | 4,489 | `9adc567c0699855a350cb045a5a7afc3fbf2e2a525a67ef1b7e349171bed58c3` |
| `fuzzy_parser_wasm_evaluation_bg.wasm` | 1,021,215 | `b6f3f2665c927c105e826085cff9b7293a24704ce57ea7ed2f278df6d51149ad` |

CJS `require` and ESM dynamic import both exposed the same function. Fresh
local load probes were 1.56 ms and 2.63 ms respectively; these are observations
on this machine, not deployment latency commitments. Missing and corrupt WASM
assets both failed before parsing with a nonzero Node exit. The generated glue
copies its `Uint8Array` argument into WASM memory. The harness restricts an
evaluation call to 1 MiB input, 64 KiB schema and 4 MiB output; its 500-row CSV
case emitted 3,481,018 bytes, demonstrating why these guards are not production
resource guarantees.

A Node Worker begins a real 25,000-row CSV parser call. The Rust boundary invokes
an inline JS entry callback immediately before `parse_document_with_plan`; the
parent observes that shared signal, waits 20 ms while observing no completion
message, then terminates and reaps the Worker in 2.60 ms. This proves parser
entry plus Worker isolation/termination. It does not prove true in-call
cancellation or deadlines: the synchronous WASM call itself remains
noninterruptible on Node's event loop.

## Recommendation and limits

Independent review accepted **one Node WASM package with Worker isolation** as
the selected backend for #18. It avoids the CLI candidate's executable,
per-call process, pipe and temporary-file packaging while preserving exact byte
path JSON, safe errors and source evidence in this bounded experiment. Do not
ship a CLI fallback or a second backend.

This does not prove browser, Next.js, Vercel or deployed-runtime behavior. It
does not establish #17's cell, record, schema or output limits, secure sandboxing,
true cancellation inside a synchronous call, package installation, artifact
identity/version policy, or a public TypeScript contract. Generated `target`,
`.toolchain` and `pkg` paths are intentionally ignored and must not be staged.

# 0006 — Offer A Library Interface; Require Evidence For Backend Selection

## Status

Library-first direction and bounded evidence independently reviewed, 2026-08-28.
Backend selection remains open in [#11](https://github.com/2001J/fuzzy-parser/issues/11).
Proposed caller boundary: **one small generic JavaScript/TypeScript library
interface, running with the caller and no queue or separately operated service**.
Simplicity of installation, calling and deployment is an explicit criterion.
The backend is **not finally selected**: a bundled CLI is the only executed
prototype; WebAssembly remains a credible embedding candidate with a successful
Rust compilation check. #11 remains open for the evidence gate below. Neither
production adapter work nor Vercel deployment readiness is established.

This follows the evidence requirement in [ADR 0005](0005-independent-engine-consumer-validation.md).
It does not reinstate ADR 0004's unconditional sequence of further bindings.

## Context and comparison

The user wants an independently reusable library, using Photon as an analogy
for the caller experience, not an independently operated parsing system.
Independence does not require a network service. The existing CLI dispatches
text/TXT/CSV/XLSX through the format, schema and engine crates. Schema conversion
currently lives in the CLI. Reusing it allowed an experiment without copying
that logic or implementing #12; availability alone does not make it the best
long-term packaging.
The [dated evaluation](../evaluations/2026-08-28-node-cli.md) owns commands,
measurements, consumer inspection and dated primary sources.

| Option | Installation, calling and deployment tradeoffs | Evidence and disposition |
| --- | --- | --- |
| Packaged CLI process | A small JS wrapper can present a library call and reuse all current adapters. It still adds native artifact selection, executable permissions, process startup/reaping, bounded pipes and private temporary files. It is local to the caller, not a remote service or queue. | Only invocation prototype built; exact parity on macOS and isolated emulated Linux. Viable candidate if its packaging burden is acceptable; not a final default |
| WebAssembly in Node, possibly browsers later | Closest to the requested embedding model: could load a portable module and pass bytes without per-call executables/temp files. Requires JS exports, byte-input parity, memory/copy budgets, initialization and cancellation design. Browser delivery is not an initial requirement. | Core/schema/formats pass a WASM compilation check. TXT/CSV byte APIs exist; XLSX currently exposes only a path API, although its dependency supports in-memory readers. JS execution/package parity remains untested; not rejected |
| Native Node binding | Could avoid per-call process startup. Node-API offers ABI stability for its own interface, but a new binding still needs platform binaries, cancellation and exact contract tests. No measured throughput requirement justifies it yet. | Deferred; [Node-API documentation](https://nodejs.org/download/release/v22.14.0/docs/api/n-api.html) |
| Rust HTTP service | Avoids caller-native packaging but adds a separately operated deployment, authentication, networking, retention and operating costs. | Outside the requested initial integration; no service or message queue is authorized or required |

All options still require the same engine capability, error and resource work.
No comparison assumes names, free text or datetime execution already works.

## Proposed caller boundary

The production adapter belongs to [#18](https://github.com/2001J/fuzzy-parser/issues/18),
not the evaluation harness. Initial validation targets server-side Node.js;
browser/Edge support is not promised. Independent Rust library and standalone
CLI use remain supported. The [integration strategy](../integration-strategy.md#proposed-library-caller-experience)
owns an illustrative call, explicitly not an existing npm/package API.

- Accept raw bytes with a declared supported format, caller-owned schema and
  supported options. Original filename metadata must have defined, safe
  handling; the prototype uses fixed temporary filenames and is not that API.
- Do not require the application to operate parser infrastructure. The package
  owns runtime plumbing; the host owns caller access, its schema/mapping,
  review/correction, export and explicitly confirmed persistence. No raw input
  or results are logged by default.
- Preserve the [engine contracts](../data-contracts.md). Successful JSON,
  parser warnings and structured parser failures must remain distinct from
  adapter initialization, timeout, cancellation, limit and packaging failures.
  Never return truncated JSON as a successful or partial parse.
- Reuse the shared schema/extraction/core path once #12 provides it. The
  experiment deliberately uses the existing CLI conversion unchanged.
  Do not introduce a second interpretation of the caller schema.
- Pin parser artifact identity as well as accepted parser/schema/contract
  versions. The current version fields alone do not distinguish local commits
  within `0.1.0`. Reject unsupported identity/contract combinations. Rollback
  selects a previous verified artifact, not a different parser or silent fallback.
- Ship no consumer-specific profile, identifier, default or dependency in
  runtime behavior. The two evaluation profiles live only under `fixtures/`.

If the CLI candidate is selected, #18 must hide process plumbing behind that
interface: use a trusted packaged executable and argument arrays with no shell,
private per-call files, bounded streams, and close/reap before cleanup. Do not
accept executable paths or command fragments from imported content. This still
has operational costs even though the caller writes a library call.

If WASM is selected, it must use the same shared schema and parsing path with
byte input; it must not introduce a second parser to bypass missing engine APIs.
The existing XLSX path reader cannot work on ordinary `wasm32-unknown-unknown`
without adaptation, but that is not a dependency-level impossibility. The
[bounded WASM probe](../evaluations/2026-08-28-node-cli.md#bounded-wasm-feasibility-check)
records the actual gap. No WASM binding was implemented here.

There is no new public TypeScript API in this decision. #18 must settle its
exact types against #12's shared schema contract; the harness is neither an
installable package nor a production wrapper.

## Backend decision gate — open

Prefer an in-process WASM package **if** the remaining byte-input, contract,
packaging and resource evidence supports its simpler installation/deployment.
Keep the measured bundled-process path as a candidate, not a silent runtime
fallback. No application should have to implement or maintain both.

Before #18 starts, a separately authorized follow-up to **#11** must test a
minimal JS/WASM call after #12 provides shared schema compilation and
[#22](https://github.com/2001J/fuzzy-parser/issues/22) provides generic XLSX byte
input. #22 is a separate `parser-formats` slice, not an implicit expansion of
#12 or #18. Compare the same four
input forms, two supported profiles, source/errors, emitted package size,
initialization, memory copies, cancellation and generic Node/Next.js packaging
against this retained CLI evidence. Do not build a second production adapter
or expand #12 in this evaluation. No XLSX byte API is implemented here.

Then amend this ADR to select **one** backend. If WASM fails a required gate,
record the evidence and evaluate whether the process packaging satisfies it.
If either choice needs another engine capability or target access, keep that
gate open. The current compilation check cannot settle JS runtime or packaging
behavior; closing #11 as a completed selection now would overstate the evidence.

## Budgets and outstanding gates

The experiment caps input at 1 MiB, schema JSON at 64 KiB, stdout at 4 MiB,
stderr at 64 KiB and each child at five seconds. These are **evaluation guards**,
not implemented engine limits or promised production capacity. Output can be
hundreds of times larger than the input. #17 must establish cell/record/schema
and output budgets before #18 advertises supported sizes.

| Gate | Owner and acceptance |
| --- | --- |
| Generic engine prerequisites | #2/#12/#17: safe errors, one schema compiler and enforced resource limits; #13–#16 remain required for final capability parity |
| One backend selected | #11 follow-up above; reviewed decision required before #18. Current WASM evidence is compilation/source inspection only |
| Package installation | #18: test an installable local package with QualEvents absent and no consumer build-time Rust toolchain; verify artifact/contract identity and missing/wrong artifact failures. For CLI, prove OS/architecture/ABI and executable mode. For WASM, prove emitted module/glue loading and byte/source parity. Implement only the selected branch |
| Runtime lifecycle | #18: cancel while processing, deadline enforcement, bounded memory/output, malformed output/version mismatch, concurrent calls and no input/credential leakage. CLI also needs kill escalation/reaping/file cleanup; WASM needs an evidenced interruption/isolation strategy. The prototype covers only a subset |
| Framework packaging | #18: a generic Next.js packaging fixture must retain the selected executable or WASM assets and invoke the installed package. It must not depend on QualEvents. No framework build was performed in #11 |
| Specific Vercel compatibility claim | Separately authorized nonproduction check: actual Node major, selected artifact loading, request/response limits, cancellation, cold/warm behavior and configured compute settings; for CLI also CPU architecture, dynamic libraries, executable permissions and scratch space. Local Linux is not this proof |
| Broad independent conformance | #19: full supported capability matrix and two unrelated profiles with the first consumer absent. The narrow #11 experiment is evidence, not completion of this gate |

Unknown Vercel project settings do not block generic engine readiness or a
qualified Node adapter. They block claiming that a particular host deployment
is ready. Host adoption, UI, migration and cutover remain external work in
[integration strategy](../integration-strategy.md).

## Consequences

The useful CLI evidence is retained, but it does not override the user's library
and deployment simplicity requirement. WASM's apparent packaging advantage is
a reason to finish its bounded evidence gate, not a claim that it is ready.
Only one production backend should be implemented after review. No service,
queue, release, deployment, host migration or publication is authorized here.

# 0006 — Offer A Library Interface; Require Evidence For Backend Selection

## Status

Library-first direction and bounded evidence independently reviewed, 2026-08-28.
The corrected 2026-08-30 #11 evaluation was independently reviewed and selects
Node WASM with Worker isolation; [the dated evidence](../evaluations/2026-08-30-wasm-runtime.md)
records its scope and limits. [#11](https://github.com/2001J/fuzzy-parser/issues/11)
owns the completed selection, not production-adapter readiness.
Proposed caller boundary: **one small generic JavaScript/TypeScript library
interface, running with the caller and no queue or separately operated service**.
Simplicity of installation, calling and deployment is an explicit criterion.
The reviewed #11 evidence selects **one Node WASM package with Worker
isolation** as the single backend for #18. Neither production adapter work nor
Vercel deployment readiness is established.

This follows the evidence requirement in [ADR 0005](0005-independent-engine-consumer-validation.md).
It does not reinstate ADR 0004's unconditional sequence of further bindings.

Follow-up, 2026-08-28: [#22's XLSX byte API](../data-contracts.md#xlsx-library-input--implemented)
is now implemented and independently verified locally. The comparison below
records the #11 baseline; its path-only XLSX limitation is addressed by that
separate slice. The bounded JS/WASM runtime evidence is recorded below.

Further follow-up, 2026-08-28: [#12's shared schema compiler and core plan](../data-contracts.md#executable-schema)
are independently reviewed and locally integrated. The private CLI conversion
described at the original baseline below has been replaced. Both prerequisite
APIs are available and were exercised by the bounded JS/WASM runtime evaluation
recorded below.

## Context and comparison

The user wants an independently reusable library, using Photon as an analogy
for the caller experience, not an independently operated parsing system.
Independence does not require a network service. The existing CLI dispatches
text/TXT/CSV/XLSX through the format, schema and engine crates. At the original
#11 evaluation baseline, schema conversion lived in the CLI. Reusing it allowed
that experiment without copying the logic or implementing #12; availability
alone does not make it the best long-term packaging.
The [dated evaluation](../evaluations/2026-08-28-node-cli.md) owns commands,
measurements, consumer inspection and dated primary sources.

| Option | Installation, calling and deployment tradeoffs | Evidence and disposition at the #11 baseline |
| --- | --- | --- |
| Packaged CLI process (historical baseline) | A small JS wrapper can present a library call and reuse all current adapters. It adds native artifact selection, executable permissions, process startup/reaping, bounded pipes and private temporary files. It is local to the caller, not a remote service or queue. | Historical child-process prototype with stdout/stderr guards and exact parity; retained as comparison evidence, not the selected backend |
| WebAssembly in Node, possibly browsers later | Closest to the requested embedding model: loads a portable module and passes bytes without per-call executables/temp files. Requires JS exports, byte-input parity, memory/copy budgets, initialization and cancellation design. Browser delivery is not an initial requirement. | #11 exercised TXT/CSV/XLSX bytes through CJS and ESM, exact native parity, source/errors, packaging failures and Worker termination. Recommended as one backend with Worker isolation; deployment and package-install gates remain open |
| Native Node binding | Could avoid per-call process startup. Node-API offers ABI stability for its own interface, but a new binding still needs platform binaries, cancellation and exact contract tests. No measured throughput requirement justifies it yet. | Deferred; [Node-API documentation](https://nodejs.org/download/release/v22.14.0/docs/api/n-api.html) |
| Rust HTTP service | Avoids caller-native packaging but adds a separately operated deployment, authentication, networking, retention and operating costs. | Outside the requested initial integration; no service or message queue is authorized or required |

All options still require the same engine capability, error and resource work.
No comparison assumes names, free text or datetime execution already works.

## Proposed caller boundary

The production adapter belongs to [#18](https://github.com/2001J/fuzzy-parser/issues/18),
not the evaluation harness. Initial validation targets server-side Node.js;
browser/Edge support is not promised. Independent Rust library and standalone
CLI use remain supported. The [integration guide](../integration-strategy.md#recommended-application-flow)
owns the current application-facing call pattern; this ADR retains the
historical selection rationale.

- Accept raw bytes with a declared supported format, caller-owned schema and
  supported options. Original filename metadata must have defined, safe
  handling; the evaluation passes the optional filename directly to byte readers.
- Do not require the application to operate parser infrastructure. The package
  owns runtime plumbing; the host owns caller access, its schema/mapping,
  review/correction, export and explicitly confirmed persistence. No raw input
  or results are logged by default.
- Preserve the [engine contracts](../data-contracts.md). Successful JSON,
  parser warnings and structured parser failures must remain distinct from
  adapter initialization, timeout, cancellation, limit and packaging failures.
  Never return truncated JSON as a successful or partial parse.
- Reuse the now-implemented #12 shared schema/extraction/core path. The original
  experiment deliberately used its baseline CLI conversion unchanged.
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
The original XLSX path-only API could not work on ordinary `wasm32-unknown-unknown`
without adaptation; #22 has since added the required byte reader. The historical
[bounded WASM probe](../evaluations/2026-08-28-node-cli.md#bounded-wasm-feasibility-check)
records that baseline gap. The evaluation binding is not a production package
and must not be treated as one.

There is no new public TypeScript API in this decision. #18 must settle its
exact types against #12's shared schema contract; the harness is neither an
installable package nor a production wrapper.

## Backend decision — Node WASM with Worker isolation

The corrected and independently reviewed #11 evidence selects an in-process
WASM package with Worker isolation as the one backend. Keep the measured bundled-process
path as evidence only, not a silent runtime fallback. No application should
implement or maintain both.

The corrected follow-up tested the delivered #12 shared schema compiler and
[#22](https://github.com/2001J/fuzzy-parser/issues/22) generic XLSX byte input.
It compared four input forms, two profiles, source/errors, package size,
initialization, memory copies and Worker termination against the retained CLI
evidence. #18 must independently prove package installation, artifact identity,
resource limits and host packaging before implementation. Do not build a second
adapter or expand #12 in this evaluation.

The corrected evidence selects Node WASM with Worker isolation for future #18.
The installed-package, true in-call cancellation and deadline,
artifact identity/public TypeScript API, and Next.js/Vercel/deployment gates
remain with #18 or separately authorized work. The generic #17 engine resource
limits are now integrated; #18 must enforce and expose them through the package
boundary. If any required gate fails,
record that result before implementation; do not add a silent CLI fallback.

## Budgets and outstanding gates

The experiment capped input at 1 MiB, schema JSON at 64 KiB, stdout at 4 MiB,
stderr at 64 KiB and each child at five seconds. Those remain historical
**evaluation guards**, not promised production capacity. #17 subsequently
implemented typed cell/record/schema and output budgets; #18 must preserve and
test those budgets through the installed package.

| Gate | Owner and acceptance |
| --- | --- |
| Generic engine prerequisites | #2/#12 safe errors and shared schema compiler plus #13/#14/#15/#16 capability slices and #17 enforced resource limits are delivered |
| One backend selected | Completed in #11: Node WASM with Worker isolation after independently reviewed CJS/ESM runtime parity and source/error checks |
| Package installation | #18: test an installable local package with QualEvents absent and no consumer build-time Rust toolchain; verify artifact/contract identity and missing/wrong artifact failures. For CLI, prove OS/architecture/ABI and executable mode. For WASM, prove emitted module/glue loading and byte/source parity. Implement only the selected branch |
| Runtime lifecycle | #18: true in-call cancellation/deadline policy, bounded memory/output, malformed output/version mismatch, concurrent calls and no input/credential leakage. #11 evidences Worker entry/termination only; it does not claim interruption of a synchronous call |
| Framework packaging | #18: a generic Next.js packaging fixture must retain the selected executable or WASM assets and invoke the installed package. It must not depend on QualEvents. No framework build was performed in #11 |
| Specific Vercel compatibility claim | Separately authorized nonproduction check: actual Node major, selected artifact loading, request/response limits, cancellation, cold/warm behavior and configured compute settings; for CLI also CPU architecture, dynamic libraries, executable permissions and scratch space. Local Linux is not this proof |
| Broad independent conformance | #19: full supported capability matrix and two unrelated profiles with the first consumer absent. The narrow #11 experiment is evidence, not completion of this gate |

Unknown Vercel project settings do not block generic engine readiness or a
qualified Node adapter. They block claiming that a particular host deployment
is ready. Host adoption, UI, migration and cutover remain external work in
[integration strategy](../integration-strategy.md).

## Consequences

The useful historical CLI evidence is retained, but Node WASM with Worker
isolation is the selected backend for #18. Only one production backend
should be implemented after review. No service,
queue, release, deployment, host migration or publication is authorized here.

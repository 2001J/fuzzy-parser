# Integration Strategy

Fuzzy Parser is an independent Rust project. Consuming applications provide schemas, options, and business workflows; they do not require domain logic to be compiled into the parser core.

## Integration contract

Every integration should converge on the same conceptual request:

```text
raw input
+ target schema
+ parser options
→ parse result
```

And the same conceptual response:

```text
records
+ field assignments
+ confidence
+ explanations
+ warnings
+ rejected fragments
+ provenance
```

A TypeScript application may define a guest or contribution schema, but the Rust parser only sees generic fields, aliases, constraints, and hints.

## Stage 1: CLI

The CLI is the first supported integration surface.

Reasons:

- Keeps the project independently executable.
- Makes end-to-end behavior easy to test.
- Establishes JSON contracts before language bindings.
- Supports scripts and backend process invocation.
- Avoids early native packaging complexity.

Planned commands:

```bash
fuzzy-parser inspect --input sample.txt
fuzzy-parser inspect --stdin
fuzzy-parser parse --input sample.txt --schema schema.json
```

CLI rules:

- Structured JSON on stdout for machine modes.
- Diagnostics on stderr.
- Documented non-zero exit codes.
- No business-specific command names.
- No hidden network dependency.

## Stage 2: TypeScript process integration

A TypeScript backend can initially invoke the CLI as a child process.

```text
TypeScript backend
    ↓ JSON request or explicit files
Rust CLI process
    ↓ JSON response
TypeScript review workflow
```

Advantages:

- Simple separation.
- Independent deployment and testing.
- No native binding toolchain.

Trade-offs:

- Process startup overhead.
- File or stdin coordination.
- Need for timeout and output-size limits.

This is acceptable for proving integration before optimizing it.

## Stage 3: WebAssembly

Compile the reusable parser core to WebAssembly and package it for TypeScript.

```text
Browser or Node TypeScript
    ↓
WebAssembly parser
    ↓
Parse result in memory
```

Best use case:

- Pasted text and moderate local files.
- Immediate browser preview.
- Privacy-sensitive input that should remain on the user device until confirmation.

Requirements:

- No CLI-only dependencies in the WebAssembly build.
- Stable generated or maintained TypeScript types.
- Shared fixtures proving parity with native Rust.
- Browser-safe resource limits.
- Clear memory expectations for large spreadsheets.

XLSX support may need a separate browser adapter or pre-extraction layer depending on library compatibility and bundle size.

## Stage 4: Native Node binding

A native binding may be added when server-side TypeScript applications require lower overhead than process invocation.

Use only when benchmarks justify:

- Platform-specific builds.
- Native package distribution.
- Additional CI targets.
- ABI and runtime compatibility work.

The binding must remain thin and call the same library API.

## Stage 5: HTTP service

A service makes the parser available to any language.

Potential API:

```http
POST /v1/inspect
POST /v1/parse
```

A service introduces responsibilities outside the core parser:

- Authentication.
- Rate limits.
- Upload limits.
- Timeouts.
- Input retention.
- Logging redaction.
- Deployment and scaling.
- Version routing.

Do not build the service merely because it is architecturally possible.

## Standalone application

The standalone application should be a consumer of the parser, not a second parser implementation.

It owns:

- Paste and upload interface.
- Schema editor or generic templates.
- Review table.
- Edits, approvals, rejection, split, and merge interactions.
- Export and clipboard formatting.
- Local profile storage.

It must not hide parser warnings or discard source evidence.

## Qualevents integration example

Qualevents may offer an import choice such as guest list or contribution list. That selection belongs to Qualevents.

```text
Qualevents user chooses import type
    ↓
Qualevents loads its own schema/profile
    ↓
Qualevents sends raw input + schema to Fuzzy Parser
    ↓
Fuzzy Parser returns generic structured candidates
    ↓
Qualevents displays domain-specific review UI
    ↓
Qualevents saves confirmed records and runs its own workflow
```

The Fuzzy Parser repository must not import Qualevents code or hardcode Qualevents categories.

## Type ownership

The Rust models are authoritative for parser behavior.

TypeScript types may be:

- Generated from the contract.
- Maintained manually with contract tests.
- Generated from a language-neutral schema later.

Whichever approach is selected must detect incompatible changes in CI.

## Compatibility rules

- The same fixture should produce semantically equivalent output across CLI, WebAssembly, native binding, and service surfaces.
- Integration layers may change transport representation but not silently change parsing meaning.
- Public contracts include parser and contract versions.
- Host applications decide review thresholds and blocking warnings.
- No integration surface may cause production side effects directly from an unreviewed parse result.

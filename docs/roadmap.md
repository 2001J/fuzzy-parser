# Roadmap

This roadmap describes product outcomes, not completed-ticket history. Current
capabilities belong in [Current state](current-state.md); old audits and
implementation evidence live under [Internal documentation](internal/README.md).

## Available foundation

The `development` branch contains:

- reusable Rust application profiles;
- TXT, CSV, XLSX, pasted-text, and standard-input adapters;
- deterministic candidate detection and schema assignment;
- contextual text and possible-name fields;
- reversible source evidence and unused-content tracking;
- table header, row, and sheet selection;
- typed resource limits and safe structured errors;
- a CLI and an installable, unpublished Node/WebAssembly package;
- cross-profile native, CLI, CommonJS, and ESM conformance checks;
- CI on Linux and macOS plus container and package verification.

This foundation is an engine and integration boundary. It is not a standalone
review product and does not mean any consuming application has completed its
own migration.

## Next: understand more real-world input

Prioritize generic capabilities repeatedly needed by independent consumers:

1. Locale-aware phone interpretation with caller-provided country context.
2. Currency codes, common locale formats, and caller-defined money hints.
3. Datetime execution and broader date interpretation.
4. Declared TSV and delimited-text input.
5. Legacy XLS only if a real consumer still requires it.
6. Workbook display values and additional non-executed metadata where source
   libraries can preserve them truthfully.

Each capability must retain unresolved evidence and pass cross-profile tests. A
consumer-specific shortcut is not an engine feature.

## Next: make review understandable

Build a standalone review/export experience on the existing contracts:

- paste or upload without writing a schema during each import;
- select a saved application profile;
- show a clean editable table;
- explain only the rows that need attention;
- preserve access to original and unresolved evidence;
- copy or export corrected results as TXT, CSV, or XLSX;
- keep confirmation and persistence outside the parser.

This interface should also provide a reference composition for applications
embedding the parser.

## Release path

Before the first public package release:

1. Select the release version and produce candidate artifacts.
2. Verify Rust, CLI, Node/WASM, checksums, and supported platforms.
3. Review migration notes and capability documentation.
4. Exercise installation from the exact candidate artifacts.
5. Publish only through the protected manual release workflow from `main`.

Candidate builds do not imply publication. Rust crates, npm packages, containers,
tags, and GitHub Releases require explicit authorization.

## Consumer adoption

Each application owns its own staged adoption:

1. Define and test profiles.
2. Add preview and correction without persistence.
3. Add export.
4. Add explicit confirmation through existing domain services.
5. Compare supported inputs with the legacy path.
6. Cut over only the inputs with proven parity and rollback.

Engine readiness does not authorize host deployment or data migration.

## Later

- Additional runtime surfaces justified by measured deployment needs.
- Property testing, fuzzing, and benchmarks beyond current safety regressions.
- Text-based PDF extraction, followed later by OCR.
- Optional correction-learning research with explicit privacy controls.

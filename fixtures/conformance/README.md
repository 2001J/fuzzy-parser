# Cross-profile conformance corpus

This synthetic corpus exercises one unmodified engine with two caller-owned
schemas. `attendance-profile.json` is shaped like a possible attendance import;
`inventory-profile.json` is an unrelated stock-check profile. Neither is a
compiled default, runtime selector or production schema.

| Input mode | Corpus input | Foundation evidence |
| --- | --- | --- |
| Pasted/stdin | `shared.txt` bytes | Caller labels, Unicode/interior whitespace, ambiguity and unresolved content |
| TXT path | `shared.txt` | Same semantic records through the file adapter |
| CSV path | `shared.csv` | Header-directed ownership, empty required cells and extra columns |
| XLSX path | `../xlsx/sample.xlsx` | Existing synthetic typed cells, missing-field warnings and source provenance |

The shared text/CSV inputs deliberately contain a clean first record and an
uncertain second record. Tests require deterministic CLI/native parity, exact
source-reference resolution, warnings, unassigned candidates and nonempty
unused spans for both profiles. Reusing the existing XLSX fixture avoids a
second binary corpus that would duplicate typed-cell coverage.

The corpus now runs through the native library, CLI, and installed #18 Node/WASM
package via both CJS and ESM. See the [capability matrix](../../docs/conformance.md).
It does not establish host UI, authorization, review, persistence, export,
messaging, migration, deployment or cutover.

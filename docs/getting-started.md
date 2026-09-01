# Getting Started

This guide runs Fuzzy Parser from the repository and explains the first result.
For application embedding, continue to the [integration guide](integration-strategy.md).

## 1. Inspect an input

Inspection converts a supported source into the canonical raw document without
assigning application fields:

```bash
cargo run -p parser-cli -- inspect fixtures/csv/comma.csv
```

The output preserves values and their row, column, sheet, line, or byte
locations. Inspection is useful when deciding whether an adapter sees the input
as expected.

Supported inputs are pasted text, standard input, UTF-8 TXT, CSV, and XLSX.
Legacy XLS, PDF, images, and OCR are not supported.

## 2. Parse with a profile schema

The repository includes small schemas for executable examples:

```bash
cargo run -p parser-cli -- schema validate fixtures/schema/contact.json
cargo run -p parser-cli -- parse fixtures/csv/comma.csv \
  --schema fixtures/schema/contact.json
```

The sample `contact.json` requests only an email field. The parser therefore
detects names as source content but does not pretend the application asked for
them. Use `contact_with_text.json` to exercise caller-directed text assignment.

This distinction is fundamental:

- The **application integrator** defines a reusable profile.
- The **person importing data** only supplies the input and reviews the draft.
- Fuzzy Parser never guesses the application's business model.

## 3. Read the result

A parse response contains:

```text
content
  records or sheets
    detected candidates
    fields assigned by the profile
    unassigned candidates
    review status and reasons
warnings
source_evidence
  canonical input
  source references
  unused spans
```

`needs_review` is not a fatal error. It means the application should show the
record to a person because required data is missing, candidates are ambiguous,
or evidence remains unresolved.

Read [Results and review](results-and-review.md) before building a save or import
button.

## 4. Try text input

Standard input and inline input are treated as text, not automatically as CSV:

```bash
printf 'name: Ada Lovelace\n' | \
  cargo run -p parser-cli -- parse --stdin \
  --schema fixtures/schema/contact_with_text.json

cargo run -p parser-cli -- inspect --text $'Ada Lovelace\nGrace Hopper'
```

Unlabeled residual text can remain unresolved. That is intentional: preserving
uncertainty is safer than manufacturing a name, note, or record boundary.

## 5. Understand process outcomes

| Outcome | stdout | stderr | Exit code |
| --- | --- | --- | --- |
| Data or parse draft | JSON | empty | `0` |
| Processing failure | empty | structured JSON | `1` |
| Invalid command usage | empty | plain text | `2` |
| Help | plain text | empty | `0` |

Use `cargo run -p parser-cli -- --help` for the complete command grammar.
Diagnostic mode may expose sensitive paths or caller values and must be placed
before the command:

```bash
cargo run -p parser-cli -- --diagnostics inspect missing.txt
```

## Next steps

- Define reusable fields: [Application profiles](application-profiles.md)
- Embed the parser: [Integration guide](integration-strategy.md)
- Check supported behavior: [Capability matrix](current-state.md)
- Inspect exact JSON: [Advanced data contracts](data-contracts.md)

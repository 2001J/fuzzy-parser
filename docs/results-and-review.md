# Results And Review

Fuzzy Parser returns a draft with evidence. It does not return an instruction to
save data automatically.

## Result structure

Every response identifies its parser and contract versions, source type, parsed
content, warnings, and source evidence.

For each record, applications should consider three collections:

1. **Assigned fields** — candidates selected for caller-defined fields.
2. **Unassigned candidates** — recognized values that could not be safely owned.
3. **Unused source** — original content that produced no candidate.

Showing only assigned values hides important information. A useful review UI
keeps the source beside the proposed record and makes unresolved evidence easy
to inspect.

## Review status

| Status | Meaning | Application action |
| --- | --- | --- |
| `clear` | No generic review reason was produced | Continue with application validation |
| `needs_review` | Missing, ambiguous, conflicting, or unresolved evidence exists | Ask a person to inspect or correct the draft |

Neither status is business approval. Even a clear parser record may fail an
application's permissions, uniqueness, policy, or validation rules.

## Confidence

Candidate confidence is a deterministic heuristic score explaining relative
parser evidence. It is not a calibrated probability and must not be converted
into statements such as “98% accurate” or “approved.”

Use reason codes and source evidence when explaining a suggestion. Do not base
irreversible actions on a numeric score alone.

## Source evidence

Candidate source references point into the canonical source document included
with the response. Applications can use them to:

- display the exact original value;
- highlight where an assignment came from;
- distinguish repeated identical values;
- preserve unused or excluded content;
- audit corrections without inventing file-byte offsets.

CSV/XLSX references address canonical cells or rendered typed values. They are
not necessarily byte offsets in the original archive. The caller should retain
the uploaded file separately when exact-file archival is required.

## Corrections and confirmation

A safe application flow is:

```text
parse
  -> show source and proposed fields
  -> correct or resolve uncertain values
  -> run application validation
  -> explicitly confirm
  -> persist through existing domain services
```

Parsing, previewing, cancelling, copying, or exporting a draft should not imply
database writes or messaging. Confirmation should recheck authorization and
current application rules because they may have changed since preview.

## Errors versus review reasons

- A structured **error** means processing could not produce a response.
- A **warning** means processing continued with recoverable uncertainty.
- A **review reason** explains why a record needs attention.

See [Errors and confidence](error-and-confidence-model.md) for the full model and
[Data contracts](data-contracts.md) for the exact serialized shapes.

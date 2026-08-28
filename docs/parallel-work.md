# Parallel work board

Coordinator snapshot: 2026-08-28. This document tracks assigned work and local
integration, not automatic live monitoring. GitHub issues retain acceptance
criteria; [current state](current-state.md) describes implemented capabilities
and [the roadmap](roadmap.md) owns development order.

## Outcome

Fuzzy Parser should become the independent parsing engine behind QualEvents
imports. QualEvents will supply its own schemas and own review, correction,
export and confirmed database writes. The current work does not yet connect
the applications. See [integration strategy](integration-strategy.md).

## Team and current assignments

| Owner | Assignment | Current state | Review and integration |
| --- | --- | --- | --- |
| Existing parser worker | [#2 Safe structured errors](https://github.com/2001J/fuzzy-parser/issues/2) | Implementing in the original parser checkout | Coordinator review pending; no #2 commit or merge |
| Email worker | [#15 Punctuation-adjacent email detection](https://github.com/2001J/fuzzy-parser/issues/15) | Active; isolated branch/base verified | Independent detector fix; no error-contract edits |
| TXT test worker | [#4 Permanent TXT fixtures](https://github.com/2001J/fuzzy-parser/issues/4) | Active; isolated branch/base verified | Raw extraction/fixture coverage; coordinate overlapping I/O tests with #2 |
| QualEvents preparation worker | Existing import compatibility and proposed host tickets | Active; read-only branch/base verified | No host implementation, database access or GitHub mutations |
| Coordinator | Assignments, independent review, integration and reporting | Team startup verified; implementation reports pending | Sole integration owner; does not treat worker reports as review approval |

An assigned task must not silently expand into another ticket.

Reasoning budgets are task-specific: #2 uses high for the compatibility-sensitive
error migration; the bounded #15 fix, #4 fixtures and read-only host preparation
use medium. Existing task models remain unchanged; new tasks use the user's
configured default model. Raise reasoning only
for a concrete unresolved problem or difficult review; do not use extra-high
uniformly or add idle workers merely to increase headcount.

### Verified task locations

- **Align Fuzzy Parser roadmap for QualEvents** (#2, high)
  - Task: `01a04432-a13b-7471-a1f2-3adcd2e634c7`.
  - Branch: `codex/align-fuzzy-parser-roadmap-for`.
  - Folder: `/Users/josephkoyi/Desktop/bonkers/fuzzy-parser`.
- **Fix FP-15 email boundaries** (medium)
  - Task: `01a0478b-75ed-73f0-b3eb-d5a3d3b52cb4`.
  - Branch: `codex/fp-15-email-boundaries`.
  - Folder: `/Users/josephkoyi/.codex/worktrees/c5ad/fuzzy-parser`.
- **Complete FP-4 TXT regression fixtures** (medium)
  - Task: `01a0478b-7750-7203-a772-356b98192ad9`.
  - Branch: `codex/fp-4-txt-fixtures`.
  - Folder: `/Users/josephkoyi/.codex/worktrees/fb3f/fuzzy-parser`.
- **Prepare QualEvents parser integration** (medium; read-only)
  - Task: `01a0478b-7dac-7910-a1e1-3b0973c88f5c`.
  - Branch: `codex/qualevents-parser-preparation`.
  - Folder: `/Users/josephkoyi/.codex/worktrees/2e0b/wedding-app`.

The coordinator verified the two new parser branch HEADs at
`74214884dd8ce22fec745be104c329fc33922c1f` and the host branch at
`50fcaf072abd5307157ce1e0ee96676729e896c5`. Startup is not implementation approval.

## Branches and working folders

A branch records one ticket's changes. A worktree is the separate local folder
where that branch can be edited concurrently. Workers use both: separate ticket
branches in separate worktrees of the same repository, not unrelated copies or
temporary publishing clones.

- Original Fuzzy Parser checkout: `/Users/josephkoyi/Desktop/bonkers/fuzzy-parser`.
- Current local integration branch: `codex/align-fuzzy-parser-roadmap-for`.
- Last reviewed implementation baseline before this coordination setup:
  `0f16dfce683eaf6f5b42accf2617447f7a310ed9`.
- The existing #2 worker stays in that checkout until its current slice finishes.
  Do not switch its branch, capture its unfinished changes into another worker,
  or merge another implementation into its dirty checkout.
- New parser workers start from the reviewed committed baseline plus the
  coordinator's documentation-only setup commit, not from uncommitted #2 work.
  If app setup starts from an older ancestor, only a verified fast-forward to
  that baseline is allowed before implementation; stop on divergence or dirt.
- QualEvents preparation uses a separate worktree at reviewed host commit
  `50fcaf072abd5307157ce1e0ee96676729e896c5`. The original host checkout and its
  `product/lifecycle-v2` branch remain untouched.

No worker changes another worktree or shared branch. Each keeps its own build
output. Heavy container builds, host builds, and database-backed checks require
coordinator scheduling; worktree isolation does not isolate databases, ports,
credentials, Docker, or machine resources.

## Ownership and dependencies

- #2 owns the shared error contract, schema conversions and error-facing CLI
  plumbing. Its successful-input behavior must remain unchanged.
- #15 owns the email detector and its source-reference/CLI regressions. It must
  not take over error serialization or unrelated tokenization changes.
- #4 owns durable TXT fixtures and adapter extraction tests. It must reuse
  existing coverage where appropriate, preserve raw error causes and avoid
  asserting the old leaking JSON/Display format. #2 owns that migration.
- The coordinator reconciles shared test modules and current-state/roadmap
  documentation. A clean Git merge alone does not establish semantic safety.
- #12 waits for reviewed #2. Backend selection and the production adapter remain
  gated by their existing tickets. Do not parallelize dependent implementation
  by inventing competing interfaces.
- QualEvents preparation may identify caller requirements and migration tests,
  but must not choose a parser backend, introduce domain behavior into Fuzzy
  Parser, create an integration package, or replace working import routes.

## Review-to-integration process

1. **Assigned:** define the ticket, owner, baseline, permitted files, tests and
   dependencies before editing.
2. **Implementing:** the worker changes its own branch, retains regressions and
   reports blockers without broadening scope.
3. **Awaiting review:** the worker stops and reports exact paths, baseline/HEAD,
   test outcomes, compatibility effects and remaining limits. Workers do not
   approve their own work or start another ticket.
4. **Reviewed:** the coordinator examines the actual diff and runs independent
   checks. Corrections go back to the owner. Local commits are made only after
   review or a specific coordinator instruction; no unrelated paths are staged.
5. **Integrated locally:** once the integration checkout is safe, the coordinator
   combines approved ticket commits one at a time and reruns affected combined
   tests. Conflict resolution must preserve both tickets' behavior and tests.
6. **Closed / published:** close only when the ticket's required acceptance and
   verification are complete. State whether completion is local or published.
   A push, pull request, merge into main, deployment, package or container
   publication still needs separate authorization.

Planned order: finish/review #2 in the original checkout, then integrate #15 and
#4 after their independent reviews. They can implement concurrently, but must
also pass combined checks after integration. Host preparation is a report, not
a host code merge.

## Reporting and remaining gates

The coordinator reports user-visible outcomes, active tickets, blockers and the
next milestone. Worker message traffic is not a substitute for that summary.
Record status and evidence here after meaningful review/integration transitions,
not after every command.

- #3, #10, #21 and #22 are closed for reviewed delivered work. #8 is superseded,
  not counted as delivered implementation.
- [#23 CI](https://github.com/2001J/fuzzy-parser/issues/23) is committed and tested
  locally but remains open for the first authorized GitHub-hosted run.
- At setup, reviewed parser work is local only; remote main is still
  `8f878a45d7801ab0ca0a7d10a1b8aca353c7c192`. No feature branch has been pushed.
- Full engine readiness, runtime selection and actual QualEvents adoption are
  still open. Parallelism does not relax these gates.

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
| Existing parser worker | #2 completed; available for a bounded independent review | Stopped; original checkout returned to coordinator | #2 privacy finding corrected and re-reviewed; no next implementation assigned here |
| Schema worker (previously email) | [#12 Shared schema compilation](https://github.com/2001J/fuzzy-parser/issues/12) | Preparation complete; next implementation dispatch | High reasoning; new ticket branch from the reviewed integration baseline |
| Validation worker (previously TXT tests) | [#5 Reusable file validation](https://github.com/2001J/fuzzy-parser/issues/5) | Preparation complete; next implementation dispatch | Medium reasoning; separate ticket branch and scoped error additions |
| QualEvents preparation / privacy reviewer | Host compatibility report and #2 message-invariant re-review | Complete; stopped | Read-only host work; no integration or runtime approval claimed |
| Coordinator | Assignments, independent review, integration and reporting | #2/#15/#4 code locally integrated; GitHub tickets closed | Sole integration owner; next changes must pass the same review/check gate |

An assigned task must not silently expand into another ticket.

Reasoning budgets are task-specific: #12 uses high for schema/assignment
compatibility, while #5 uses medium for bounded file validation. The prior #2
migration and privacy review used high; #15, #4 and host preparation used medium.
Existing task models remain unchanged; new tasks use the user's
configured default model. Raise reasoning only
for a concrete unresolved problem or difficult review; do not use extra-high
uniformly or add idle workers merely to increase headcount.

### Verified task locations

- **Align Fuzzy Parser roadmap for QualEvents** (#2 complete; stopped)
  - Task: `01a04432-a13b-7471-a1f2-3adcd2e634c7`.
  - Branch: `codex/align-fuzzy-parser-roadmap-for`.
  - Folder: `/Users/josephkoyi/Desktop/bonkers/fuzzy-parser`.
- **Fix FP-15 email boundaries** (#15 complete; #12 next, high)
  - Task: `01a0478b-75ed-73f0-b3eb-d5a3d3b52cb4`.
  - Completed branch: `codex/fp-15-email-boundaries`; next branch: `codex/fp-12-schema-compilation` (startup verification required).
  - Folder: `/Users/josephkoyi/.codex/worktrees/c5ad/fuzzy-parser`.
- **Complete FP-4 TXT regression fixtures** (#4 complete; #5 next, medium)
  - Task: `01a0478b-7750-7203-a772-356b98192ad9`.
  - Completed branch: `codex/fp-4-txt-fixtures`; next branch: `codex/fp-5-file-validation` (startup verification required).
  - Folder: `/Users/josephkoyi/.codex/worktrees/fb3f/fuzzy-parser`.
- **Prepare QualEvents parser integration** (preparation and privacy re-review complete; read-only)
  - Task: `01a0478b-7dac-7910-a1e1-3b0973c88f5c`.
  - Branch: `codex/qualevents-parser-preparation`.
  - Folder: `/Users/josephkoyi/.codex/worktrees/2e0b/wedding-app`.

The first parser branches started at `7421488`. Reviewed ticket commits are
`4738132` (#2), `0b8b529` (#15), and `a42ce18` (#4). Sequential local integration
finished at `51211e06c245808b03521e3e99d03d88dc6e5523`. New dispatches must use
that verified code plus the coordinator's documentation-only update, not an
older main or another worker's unfinished changes. The host worktree remains
clean at `50fcaf072abd5307157ce1e0ee96676729e896c5`.

## Branches and working folders

A branch records one ticket's changes. A worktree is the separate local folder
where that branch can be edited concurrently. Workers use both: separate ticket
branches in separate worktrees of the same repository, not unrelated copies or
temporary publishing clones.

- Original Fuzzy Parser checkout: `/Users/josephkoyi/Desktop/bonkers/fuzzy-parser`.
- Current local integration branch: `codex/align-fuzzy-parser-roadmap-for`.
- Last reviewed combined implementation baseline:
  `51211e06c245808b03521e3e99d03d88dc6e5523`.
- The original checkout is clean and coordinator-owned again. Future parser
  implementations use their assigned worktrees; no worker starts another slice
  in this checkout without an explicit assignment.
- New parser workers start from the reviewed committed baseline plus the
  coordinator's documentation-only update, never from uncommitted work.
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

- #12 owns shared schema compilation, field-scoped enum assignment and explicit
  executable-capability checks. Preserve #2 privacy, #10 source evidence and
  #15 email behavior; dependent text/name support and runtime work stay separate.
- #5 owns regular-file/extension/size/empty policy and TXT path integration.
  Default empty acceptance remains; strict extensions and new typed failures
  need the documented compatibility tests. Wider format limits remain #17.
- Both may add disjoint typed failures and tests to the shared error module.
  Neither changes the other's checkout or failure variants. The coordinator
  reconciles those additive overlaps and rechecks exact safe output.
- The coordinator reconciles shared test modules and current-state/roadmap
  documentation. A clean Git merge alone does not establish semantic safety.
- #2 is complete, so #12 and #5 can proceed independently. #6 waits for #5;
  backend selection and the production adapter remain gated by their existing
  tickets. Do not invent competing schema/runtime interfaces in parallel.
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

Completed order: #2, then #15, then #4, with checks between integrations. Next,
#12 and #5 can implement independently; reviewed changes still integrate one
at a time. Host preparation is a report, not a host code merge.

## Reporting and remaining gates

The integrated baseline passed all 183 Rust tests (no ignored tests), formatting,
locked Clippy/build/release checks, and three-library WASM compilation. Test
discovery retains all 150 tests from the email branch, all 142 from the TXT
branch, and the six original Cargo targets. Merge resolutions preserved the
complete #2 test bodies while adding the two nested test modules.

The same code passed 11 Node CI guard tests, native invocation parity (11 success
cases, 9 error cases, 7 controls, 2,476 source resolutions, 22 cleanups), and all
14 checks on a freshly built Linux/amd64 container. Local image
`fuzzy-parser-parallel-review:183-tests` has digest
`sha256:6cb6c100043d0359499d6827f44051a722dd59566d0cfd2ac7d5dad99470ce67`.
This is local evidence, not hosted CI, WASM execution, deployment or publication.

The coordinator reports user-visible outcomes, active tickets, blockers and the
next milestone. Worker message traffic is not a substitute for that summary.
Record status and evidence here after meaningful review/integration transitions,
not after every command.

- #2, #3, #4, #10, #15, #21 and #22 are closed for reviewed delivered work. #8 is superseded,
  not counted as delivered implementation.
- [#23 CI](https://github.com/2001J/fuzzy-parser/issues/23) is committed and tested
  locally but remains open for the first authorized GitHub-hosted run.
- Reviewed parser work remains local only; remote main is still
  `8f878a45d7801ab0ca0a7d10a1b8aca353c7c192`. No feature branch has been pushed.
- Full engine readiness, runtime selection and actual QualEvents adoption are
  still open. Parallelism does not relax these gates.

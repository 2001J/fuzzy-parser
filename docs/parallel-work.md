# Parallel work board

Coordinator snapshot: 2026-08-29. This document tracks assigned work and local
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
| Existing parser worker | Independent #6 CLI review; [#11 JS/WASM preparation](https://github.com/2001J/fuzzy-parser/issues/11) | Review and preparation complete; stopped, no experiment dispatched | Medium reasoning; original checkout remains coordinator-owned |
| Schema worker (previously email) | [#13 Caller-directed text and name fields](https://github.com/2001J/fuzzy-parser/issues/13) | Complete; independently reviewed, integrated and closed | High reasoning for source ownership and abstention |
| Validation worker (previously TXT tests) | [#7 TXT subprocess matrix](https://github.com/2001J/fuzzy-parser/issues/7) | Complete; independently reviewed, integrated and closed | Medium reasoning; no production behavior changed |
| QualEvents preparation / privacy reviewer | Host compatibility report and #2 message-invariant re-review | Complete; stopped | Read-only host work; no integration or runtime approval claimed |
| Coordinator | Assignments, independent review, integration and reporting | #2/#15/#4/#5/#12/#6/#7/#13 locally integrated and closed | Sole integration owner; new dispatches use the final reviewed combined baseline |

An assigned task must not silently expand into another ticket.

Reasoning budgets are task-specific: #13 uses high for source/assignment
compatibility, while #7 uses medium for bounded fixture/contract tests. The prior #2
migration and privacy review used high; #15, #4 and host preparation used medium.
Existing task models remain unchanged; new tasks use the user's
configured default model. Raise reasoning only
for a concrete unresolved problem or difficult review; do not use extra-high
uniformly or add idle workers merely to increase headcount.

### Verified task locations

- **Align Fuzzy Parser roadmap for QualEvents** (#2 and reviews complete; #11 preparation only)
  - Task: `01a04432-a13b-7471-a1f2-3adcd2e634c7`.
  - Branch: `codex/align-fuzzy-parser-roadmap-for`.
  - Folder: `/Users/josephkoyi/Desktop/bonkers/fuzzy-parser`.
- **Fix FP-15 email boundaries** (#15/#12/#13 complete)
  - Task: `01a0478b-75ed-73f0-b3eb-d5a3d3b52cb4`.
  - Current branch: `codex/fp-13-text-name-fields`; completed branches retained: `codex/fp-12-schema-compilation`, `codex/fp-15-email-boundaries`.
  - Folder: `/Users/josephkoyi/.codex/worktrees/c5ad/fuzzy-parser`.
- **Complete FP-4 TXT regression fixtures** (#4/#5/#6/#7 complete)
  - Task: `01a0478b-7750-7203-a772-356b98192ad9`.
  - Current branch: `codex/fp-7-txt-subprocess-matrix`; completed branches retained: `codex/fp-6-cli-contract`, `codex/fp-5-file-validation`, `codex/fp-4-txt-fixtures`.
  - Folder: `/Users/josephkoyi/.codex/worktrees/fb3f/fuzzy-parser`.
- **Prepare QualEvents parser integration** (preparation and privacy re-review complete; read-only)
  - Task: `01a0478b-7dac-7910-a1e1-3b0973c88f5c`.
  - Branch: `codex/qualevents-parser-preparation`.
  - Folder: `/Users/josephkoyi/.codex/worktrees/2e0b/wedding-app`.

The first parser branches started at `7421488`. Reviewed ticket commits are
`4738132` (#2), `0b8b529` (#15), and `a42ce18` (#4). Sequential local integration
finished at `51211e06c245808b03521e3e99d03d88dc6e5523`. The host worktree remains
clean at `50fcaf072abd5307157ce1e0ee96676729e896c5`.

Both #12/#5 implementation branches were verified at starting HEAD
`782ccd43a3deb5a5b2ffa3dc773f0c980996444a` (reviewed code plus coordinator docs).
#5 is committed at `74a7576fc2783adcc767ead6d131a9a4ef272bb0` and integrated at
`0d0a949faffeefc59ef209843811d1f41c4b0963`. #12 is committed at
`93229f15cdd1993593eef7d8e26400bf4c7f4cd5` and integrated at
`832e4f5816f506ef25c2796942eb265c3b122d22`. #6 and #13 both verified clean
startup at its documentation follow-up `d757e28413d91aeb1aa3e75373268199cd29ee8a`.
#6 is now committed at `e594794408741c9210d0529e3e41e0e71a2a24da` and integrated
at `21cc208dc06b6cd594c247cbd2626f69e0e4bf68`. #13 is committed at
`5ad64ef` and integrated at `31e41dd`; #7 is committed at `166f6e4` and integrated
at `c1c8845`. The coordinator explicitly reconciled shared #6/#7/#13 tests and
documentation instead of trusting clean automatic merges.

## Branches and working folders

A branch records one ticket's changes. A worktree is the separate local folder
where that branch can be edited concurrently. Workers use both: separate ticket
branches in separate worktrees of the same repository, not unrelated copies or
temporary publishing clones.

- Original Fuzzy Parser checkout: `/Users/josephkoyi/Desktop/bonkers/fuzzy-parser`.
- Current local integration branch: `codex/align-fuzzy-parser-roadmap-for`.
- Last reviewed combined implementation baseline:
  `c1c8845`.
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

- #6's complete argument grammar, explicit routing, help and TXT-only overrides
  are delivered. #7's completed real-file TXT subprocess matrix reuses #4
  fixtures and maps retained #6 tests without changing CLI/reader behavior,
  policy, dependencies or Cargo targets.
  Wider same-handle validation and resource policy remain #17.
- #13 delivered caller-directed text/name extraction through #12's shared plan.
  Unlabeled residual text remains unresolved evidence, not an inferred identity.
  New assignments cannot overlap existing assigned evidence. Preserve all old
  supported profiles; #14 segmentation and #16 table options remain separate.
- Both preserve #2 privacy, #10 source evidence, #15 email behavior and #12
  schema compilation. #13 does not rewrite #6 CLI argument handling. #7 does
  not inspect #13's draft or expand into parser capability tests.
- The coordinator reconciles shared test modules and current-state/roadmap
  documentation. A clean Git merge alone does not establish semantic safety.
- #11's bounded JS/WASM preparation is complete, but no binding, tool download,
  experiment or production adapter is dispatched by this board. #12 and #22
  prerequisites are delivered; JS execution, packaging and lifecycle evidence
  remain open. Do not invent competing schema/runtime interfaces.
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

Completed order: #2, then #15, then #4, then #5, then #12, then #6, then #13,
then #7, with checks between integrations. Reviewed changes still integrate one
at a time. Host preparation is a report, not a host code merge.

## Reporting and remaining gates

The earlier `51211e0` baseline passed all 183 Rust tests (no ignored tests), formatting,
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

The `0d0a949` baseline added independently reviewed #5 validation. All
202 macOS Rust tests passed with no ignored tests, retaining every baseline
name and the six original Cargo targets. Linux/amd64 passed 203 tests: the extra
test creates a real non-UTF-8 filename unavailable on macOS. Formatting, locked
Clippy/tests/build/release, three-library WASM compilation, 11 Node guard tests
and native parity (11 successes, 9 expected errors, 7 controls, 2,476 source
resolutions, 22 cleanups) passed. Original-checkout checks were rerun after the
only merge conflict, in the roadmap; code/fixtures match the reviewed branch.

All 14 container checks passed on `fuzzy-parser-file-validation-review:local`,
digest `sha256:1f91679a8058b5266468abd53f89a73db2b2f5d52529cc1d182327b10ceb2b0e`.
The separate Linux test run had no network and mounted only synthetic fixtures
read-only. Both runs finished; no evaluation container remains running.

The `832e4f5` baseline added independently reviewed #12 compilation and
field-scoped enums. All 232 macOS Rust tests and 233 Linux/amd64 tests passed,
with no ignored tests. The combined names are exactly the union of both ticket
suites; all six Cargo targets remain. Formatting, locked Clippy/tests/build,
release build, three-library WASM compilation, 11 Node guard tests and native
parity (11 successes, 9 errors, 7 controls, 2,476 source resolutions, 22 cleanups)
passed. The 12-case schema golden also matched the pre-#12 binary independently.
The only merge conflict was the roadmap; source overlaps were reviewed. The
final fixture include uses `CARGO_MANIFEST_DIR`, with the anchored test file
included read-only in the final isolated Linux run.

All 14 checks passed on the fresh `fuzzy-parser-schema-validation-review:local`
image, digest `sha256:0c9131bb47e49856bab64619dfac8ae4f3b14e546a8e6ed95b3af255e78aa563`.
No evaluation container remains running. These results are local verification,
not hosted CI or execution of WASM code. Documentation-only coordination updates
do not require another image build.

The current `21cc208` baseline adds independently reviewed #6 CLI handling.
All 244 macOS / 245 Linux tests passed, retaining all 232 earlier names and the
six Cargo targets. Two source reviews found no actionable issue; the second
also ran 16 bounded real-binary privacy/grammar probes. Formatting, locked
Clippy/tests/build/release, three-library WASM compilation, all 11 Node guards,
native parity and 106 documentation links passed. The new README TXT override
example exactly matched prior successful stdout. Integration had no conflict
and its tree exactly matches the reviewed ticket; original-checkout Rust checks
and native parity were rerun. No #13 draft was included.

All 14 fresh container checks passed on `fuzzy-parser-cli-contract-review:local`,
digest `sha256:0a30a1f8d5d2442dda32dde291097698d37302ddfe7fae8ecef344ee957078a6`.
The isolated Linux run had no network and only synthetic fixtures mounted
read-only; both runs finished. Native release SHA-256 is
`1b6867cf0ebd29b204e2087653da8da684c59438bf431bec5c6a885ac6150e44`.
The full TXT fixture matrix is completed in #7; expanded input/resource policies
remain #17. No hosted CI or WASM execution is implied by these local results.

The current `c1c8845` baseline combines independently reviewed #13 contextual
text/name support and #7's test-only TXT subprocess matrix. All 274 macOS and
275 Linux/amd64 Rust tests passed; Linux has one additional platform-specific
non-UTF-8 filename test. Every pre-#13 test name and all six Cargo targets remain.
Formatting, locked Clippy/tests/build, release build, three-library WASM
compilation, all 11 Node guards and native parity passed. Native parity covers
13 successes, seven expected parser failures, seven boundary controls, 2,486
source-reference checks and 22 temporary-directory cleanups. Release CLI
SHA-256 is `cf7de04a3a0e25baddc33fd6f4dc0e58886db765caedbfb2fd1a687c89edab0`.

All 14 checks passed on the fresh Linux/amd64 image
`fuzzy-parser-fp7-fp13-review:local`, digest
`sha256:877b389c846a4199c0e4a409d5615a281e33f0866767664263dbd4bf812f4fa3`.
The Linux test container had no network and mounted only synthetic fixtures
read-only. No parser evaluation container remains running. These results are
local verification, not hosted CI, WASM execution, deployment or publication.

The coordinator reports user-visible outcomes, active tickets, blockers and the
next milestone. Worker message traffic is not a substitute for that summary.
Record status and evidence here after meaningful review/integration transitions,
not after every command.

- #2, #3, #4, #5, #6, #7, #10, #12, #13, #15, #21 and #22 are closed for reviewed delivered work. #8 is superseded,
  not counted as delivered implementation.
- [#23 CI](https://github.com/2001J/fuzzy-parser/issues/23) is committed and tested
  locally but remains open for the first authorized GitHub-hosted run.
- Reviewed parser work remains local only; remote main is still
  `8f878a45d7801ab0ca0a7d10a1b8aca353c7c192`. No feature branch has been pushed.
- Full engine readiness, runtime selection and actual QualEvents adoption are
  still open. Parallelism does not relax these gates.

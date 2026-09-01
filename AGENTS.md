# Agent Working Rules

This file is the project rulebook for coding agents. Read it before making changes.

## Working Style

Separate ticket selection from ticket execution.

When choosing the next ticket, keep exploratory or triage replies plain and concise. Do not force the structure below into direction-setting, status updates, or open-ended planning.

When executing a chosen issue, sub-issue, ticket, or slice, write or restate the defined work in this structure before implementation:

```md
# Goal
[user-visible outcome]

# Success criteria
[what must be true before the final answer]

# Tests
[explicit checks required, or why the user intentionally scoped them out]

# Constraints
[architecture, compatibility, safety, evidence, and side-effect limits]

# Output
[expected files, behavior, and final summary]

# Stop rules
[when to retry, fallback, abstain, ask, or stop]
```

Use this block for defined implementation work only. Do not repeat it while the user is still deciding direction.

Concrete implementation tickets must include a `Tests` section. Tests are part of the ticket, not a separate optional pass, unless the user explicitly scopes the work as planning, documentation-only, or implementation without tests.

Use plain language. Explain process terms the first time they appear, for example CI [automated checks that run after code changes].

## Source Of Truth

Treat running code and executable tests as the source of truth. Documentation describes intended contracts, but planned behavior must not be presented as implemented behavior.

Read these before changing behavior:

- `Cargo.toml`
- `crates/**/Cargo.toml`
- `crates/**/src/**`
- `.github/workflows/ci.yml`
- `docs/current-state.md`
- `docs/product-direction.md`
- `docs/architecture.md`
- `docs/parsing-pipeline.md`
- `docs/data-contracts.md`
- `docs/release-and-environment-strategy.md`

If code and documentation disagree, verify the code, update the relevant documentation, and call out the mismatch in the final answer.

## Current Project Contract

The project is an independent, domain-neutral Rust parsing engine. It may later be embedded into TypeScript products such as Qualevents, but the parser core must not know what a guest, pledge, wedding, payment campaign, or invitation is.

Preserve these contracts unless an explicit ticket changes them:

- The parser receives raw input plus caller-provided structure or schema.
- Business meaning is injected by the consuming application rather than hardcoded into parser crates.
- Original input is preserved and remains traceable through normalization, segmentation, extraction, and assignment.
- The parser may return ambiguity, low confidence, rejected fragments, and unresolved fields. It must not fabricate certainty.
- Input adapters convert source formats into a canonical document representation. They do not perform business-specific parsing.
- Parsing stages remain separable and testable: extraction, normalization, segmentation, candidate detection, schema assignment, validation, and result construction.
- Structured errors represent failures that prevent processing. Structured warnings represent recoverable uncertainty or record-level problems.
- The CLI is the first integration surface. TypeScript, WebAssembly, native bindings, and service interfaces come later and must reuse the same parser core.

## Crate Boundaries

Keep dependencies directional and narrow:

- `parser-core`: canonical models, normalization, segmentation, candidate extraction, assignment, confidence, warnings, and parse orchestration.
- `parser-formats`: TXT, pasted text, CSV, XLSX, and future source adapters. It may depend on shared core models but must not contain domain rules.
- `parser-schema`: schema models, schema validation, field definitions, aliases, caller-provided constraints, and schema versioning.
- `parser-cli`: command parsing, file/stdin handling, JSON input/output, exit codes, and human-facing CLI errors. It must not duplicate parser logic.

Do not create circular crate dependencies. Do not place shared models in the CLI because they are first needed there.

## Change Rules

- Prefer narrow vertical slices that produce one working path end to end.
- Do not begin fuzzy heuristics before the underlying input and data contracts are stable enough to test.
- Do not mix file extraction, normalization, field assignment, and export behavior in one ticket unless the ticket explicitly defines that vertical slice.
- Do not silently discard source text, empty cells, rejected fragments, unassigned values, or warnings.
- Do not overwrite raw values with normalized values. Store both or preserve a reversible source reference.
- Do not hardcode Tanzania, WhatsApp, Qualevents, invitation categories, or pledge logic in parser-core. Such behavior belongs in caller configuration, schemas, profiles, or external adapters.
- Do not add OCR, PDF interpretation, machine learning, or LLM dependencies before deterministic TXT, CSV, XLSX, schema, and review contracts exist.
- Avoid speculative abstractions. Introduce a trait or generic only when at least two real implementations need it or a documented boundary requires it.
- Prefer explicit models and typed errors over loosely structured maps and string-only failures.
- Public JSON contracts must be versioned or changed deliberately. Add compatibility tests before changing serialized field names or meanings.
- Do not use destructive git commands unless the user explicitly asks.

## Input And Privacy Rules

- Treat all imported content as untrusted and potentially sensitive.
- Apply configurable limits for file size, line length, cell count, record count, schema size, and recursion or nesting where relevant.
- Never execute formulas, macros, embedded scripts, or external links from uploaded documents.
- Spreadsheet adapters may read displayed or stored values, but must not evaluate arbitrary workbook code.
- Avoid logging full input by default. Tests should use synthetic fixtures, not real guest or customer data.
- Error messages may identify locations and types but should avoid reproducing entire sensitive records unless the caller explicitly requests diagnostic output.

## Release And Compatibility Rules

- `development` is the long-lived integration branch. Only reviewed and verified work should be merged there.
- `main` remains the stable branch. Pull requests from `development` to `main` are opened deliberately after integration verification.
- Feature branches should be short-lived and named by ticket and purpose, using the `codex/` prefix.
- The first releases are pre-1.0. Breaking changes are allowed only when documented and covered by migration notes where users could already depend on the contract.
- Release artifacts may eventually include a Rust library, CLI binary, npm/WebAssembly package, and service image. Do not assume all surfaces must ship in the same ticket.
- Keep parser behavior deterministic for the same input, schema, configuration, and version.
- Do not publish crates, npm packages, containers, tags, or releases unless the user explicitly requests a release action.

## Review And Verification

For documentation-only changes:

- Review the diff.
- Run `git diff --check` when a local checkout is available.
- Verify links and filenames match the repository.

For Rust code changes, run checks appropriate to the slice:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace
```

Additional checks by risk:

- CLI behavior: integration tests that execute the binary and assert stdout, stderr, and exit codes.
- Serialization contracts: JSON round-trip and snapshot or golden-file tests.
- Input adapters: fixture-based tests using real files under `fixtures/`.
- Parser heuristics: regression fixtures for every discovered bug.
- Robustness: property-based tests or fuzzing for malformed and random input.
- Performance: benchmarks only after a correct path exists; do not optimize based on guesses.

When a concrete ticket changes behavior, the final answer must name the tests run or explain the explicit user-approved reason tests were out of scope.

## Documentation Discipline

Update documentation in the same slice when behavior, contracts, supported formats, release strategy, or architectural decisions change.

Use the documentation by purpose:

- `docs/current-state.md`: only what is implemented now.
- `docs/product-direction.md`: product purpose, users, boundaries, and intended capabilities.
- `docs/architecture.md`: components, crate ownership, dependency direction, and deployment shapes.
- `docs/parsing-pipeline.md`: stage-by-stage parser responsibilities and invariants.
- `docs/data-contracts.md`: canonical models and public request/response contracts.
- `docs/error-and-confidence-model.md`: errors, warnings, ambiguity, provenance, and confidence.
- `docs/testing-strategy.md`: testing layers, fixtures, regressions, fuzzing, and benchmarks.
- `docs/roadmap.md`: milestone order and release slices.
- `docs/release-and-environment-strategy.md`: branches, environments, artifacts, and release discipline.
- `docs/integration-strategy.md`: CLI, TypeScript, WebAssembly, native, and service integration.
- `docs/decisions/`: architecture decision records [ADRs].

Do not copy the same explanation into multiple documents. Link to the authoritative document instead.

## Token And Automation Discipline

- Inspect the relevant files before proposing a fix.
- Do not ask an agent to explore the entire repository when a narrower read is enough.
- If the user asks what should come next, answer with a concise next-slice recommendation first. Define the structured ticket only after the user chooses it or asks to proceed.
- Treat repeated deterministic work as a candidate for a small script, but do not create scripts before the sequence actually repeats.
- Keep judgment work outside scripts: architecture decisions, confidence rules, fixture interpretation, risk assessment, and ticket selection require reasoning.
- Utilities must be safe by default, operate on explicit paths, print their actions, fail on errors, and never publish or release without explicit instruction.
- Avoid broad restatements, speculative refactors, and framework work that does not unlock the next vertical slice.
- Stop and ask when a requested change would break a public contract, discard source data, introduce a domain-specific rule into the core, or risk publishing artifacts unexpectedly.

## Workspace And Git Hygiene

- Keep the original repository checkout as the integration and publishing location.
- When the user authorizes parallel work, the coordinator may dispatch bounded tickets to separate Git worktrees of this same repository. Give each worker its own short-lived `codex/` branch and an explicitly verified, reviewed baseline; never include another worker's unfinished changes.
- Workers edit only their assigned worktree and scope. They must not reset, switch, stage, or modify another worker's checkout. Worktrees share Git metadata, so branch operations still require coordination.
- Read the coordinator's assignment and [parallel work board](docs/parallel-work.md) for ownership and dependencies. A listed future ticket is not authorization to start it.
- Workers report their exact diff, test evidence, branch, baseline, and commit status for independent review. They do not integrate into the shared branch or publish unless separately authorized.
- The coordinator integrates reviewed changes one ticket at a time, resolves overlaps explicitly, and reruns the affected combined checks before declaring integration complete. Do not integrate into a checkout with another worker's unfinished changes. Separate non-overlapping documentation commits require explicit-path review and must leave that work untouched.
- Inspect `git status --short --branch` before staging or committing.
- Stage explicit paths when unrelated work exists.
- Do not copy changes into another clone merely to commit or push them.
- After every commit or push, verify the branch, remote state, and working tree.
- Do not commit generated build output such as `target/`.
- Do not commit real imported datasets, credentials, local environment files, or private schemas.

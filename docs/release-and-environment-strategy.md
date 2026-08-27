# Release And Environment Strategy

## Branch roles

### `main`

`main` is the stable integrated development branch.

- Pull requests target `main` unless an explicit release process says otherwise.
- Only reviewed and verified work should be merged.
- `main` is not permission to publish packages automatically.

### Feature branches

Use short-lived branches named by purpose:

```text
agent/add-project-documentation
feature/txt-reader
fix/csv-empty-cell
```

Avoid long-lived parallel product branches unless the project reaches a release cadence that requires them.

### Release branches

Do not create release branches during early development by default. Tags and reproducible artifacts should be preferred once releases begin.

If a maintenance branch is later required, document its support window and merge-back policy before creating it.

## Versioning

The current workspace/package version is `0.1.0`; the parse response contract and
schema contract each use `0.1`. Planning milestones are named outcomes
(`Reviewable import engine`, `Extended format and profile coverage`), not software versions.
The historical roadmap `0.1`–`0.14` sequence and TXT-only `v0.1` epic are
reconciled in [roadmap](roadmap.md); they do not establish published releases.

The project starts at `0.x` and follows semantic versioning in spirit:

- Patch: compatible bug fix or documentation improvement.
- Minor: new capability or intentional pre-1.0 contract change.
- Major: stable-contract breaking change after `1.0`.

Pre-1.0 does not mean careless. Any serialized contract already used by another project should receive migration notes when changed.

Separate versions may eventually exist for:

- Rust crates.
- CLI binary.
- JSON contract.
- npm/WebAssembly package.
- Service API.

They should be traceable to one parser implementation release.

## Environments

### Local development

Local work should use:

- Synthetic fixtures.
- Temporary files.
- Explicit input paths.
- No production customer data.
- No required network service for core parser tests.

The core Rust workspace should remain buildable and testable offline after dependencies are available.

### CI

CI [automated checks after code changes] currently runs on Ubuntu and performs:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace
```

CI must:

- Use synthetic test data.
- Avoid secrets for ordinary parser tests.
- Avoid publishing artifacts on pull requests.
- Fail on warnings and contract regressions.
- Add platform matrices only when platform-specific code or artifacts justify them.

### Preview or integration environment

The selected reusable boundary or later standalone UI/service may need a
separately authorized preview environment. The boundary is not yet chosen;
[integration strategy](integration-strategy.md) owns that evaluation.

Preview environments must:

- Use non-production storage.
- Avoid retaining uploaded content by default.
- Apply strict resource limits.
- Clearly identify parser and contract versions.
- Never share production credentials.

### Production service environment

A production parser service is optional. It is only relevant if the boundary
evaluation selects it or a later consumer requires it; it is not a prerequisite
for engine readiness.

Before it exists, define:

- Authentication and authorization.
- Input retention policy.
- Logging redaction.
- Request and file limits.
- Rate limits.
- Encryption in transit and at rest where storage exists.
- Observability without leaking raw records.
- Version routing and rollback.

## Release artifacts

Potential artifacts are released independently when ready:

### Rust crates

- Publish only reusable library crates intended for external consumption.
- Verify crate metadata, license files, README links, and packaged content.
- Run `cargo publish --dry-run` before publication.

### CLI binaries

- Build reproducibly for explicitly supported targets.
- Include checksums.
- Document minimum supported platform and usage examples.
- Keep JSON output contract stable within a release line.

### npm/WebAssembly package

- Generate or verify TypeScript declarations.
- Prove fixture parity with the Rust CLI.
- Document browser and Node support separately.
- Do not bundle CLI-only dependencies.

### Container image

- Use a minimal runtime image.
- Run as a non-root user where possible.
- Expose no implicit persistence.
- Pin parser version in the image tag.

The current `parser-cli` image is a batch artifact, not an HTTP service. The
[workflow](../.github/workflows/ci.yml) builds and smoke-tests it on pull requests
and pushes to `main`; pushes to `main` also publish
`ghcr.io/2001j/fuzzy-parser:latest`. Current publication does not provide an
immutable release tag. Consumers should pin a verified digest or future immutable
release tag; do not assume `latest` is a tested QualEvents deployment.

## Publication rules

- Do not publish crates, npm packages, containers, tags, releases, or deployments without explicit user instruction.
- A release ticket must identify artifact, version, target platforms, changelog, and verification commands.
- Publication credentials must never be committed.
- Release automation must not run on ordinary pull requests.
- Prefer dry runs before irreversible publication.

The existing main-push workflow publishes automatically, while the policy above
requires explicit publication authorization. Treat a push/merge to `main` as a
publication action requiring that authorization; a local planning change does
not provide it. This reconciliation documents the workflow/policy mismatch but
changes neither the workflow nor its triggers.

## Compatibility and rollback

Every release should identify:

- Parser version.
- JSON contract version.
- Schema contract version.
- Supported input formats.
- Known limitations.

Rollback should mean selecting a previous tagged artifact, not rewriting published history.

Deterministic fixtures should be runnable against old and new versions when evaluating a behavior change.

## Data retention policy

The library and CLI should not retain input beyond the process unless the caller explicitly writes output.

A future UI or service must document:

- Whether raw input is uploaded.
- How long it is retained.
- Whether diagnostics contain raw values.
- How users delete retained data.
- Whether parser corrections are stored for learning.

No correction-learning system should be enabled implicitly.

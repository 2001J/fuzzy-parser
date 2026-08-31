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
schema contract each use `0.1`; the separate [error contract](data-contracts.md#error-contract-01-and-migration-from-unversioned-errors)
now uses `0.1`. The error migration does not bump packages or imply publication.
Planning milestones are named outcomes
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

CI [automated checks after code changes] is test-only. The authoritative
[CI guide](ci.md) describes the Rust/platform, Node invocation, WASM compilation,
advisory and container gates, local reproduction, and hosted-verification limits.

CI must:

- Use synthetic test data.
- Avoid secrets for ordinary parser tests.
- Never publish or deploy on ordinary CI runs, including pushes to `main`.
- Fail on warnings and contract regressions.
- Add platform matrices only when platform-specific code or artifacts justify them.

### Preview or integration environment

A future library package or later standalone tooling may need a
separately authorized preview environment. [Integration strategy](integration-strategy.md)
links the #11 runtime selection and its open packaging/deployment gates; no production
adapter or deployment is established by local testing. The initial integration
does not require a separately operated service or message queue.

Preview environments must:

- Use non-production storage.
- Avoid retaining uploaded content by default.
- Apply strict resource limits.
- Clearly identify parser and contract versions.
- Never share production credentials.

### Production service environment

A production parser service is only a later possibility if separately requested
for another need. It is outside the initial #11 library integration direction
and is not a prerequisite for engine readiness.

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

- `@fuzzy-parser/node` is implemented at package version `0.1.0` with generated
  TypeScript declarations, Node 22 support, CJS/ESM entry points and one WASM
  backend. It is locally pack-installed and verified in generic Node/Next.js
  consumers; it has not been published.
- Its identity manifest pins adapter/parser/schema/contract versions, Rust source
  identity, wasm-bindgen version, and generated JS/WASM hashes.
- Browser support, Vercel deployment and publication require separate evidence
  and authorization. Do not bundle a CLI fallback or CLI-only dependencies.

### Container image

- Use a minimal runtime image.
- Run as a non-root user where possible.
- Expose no implicit persistence.
- Pin parser version in the image tag.

The current `parser-cli` image is a batch artifact, not an HTTP service. The
[workflow](../.github/workflows/ci.yml) builds/loads and tests it locally on its
runner; it does not log in to a registry or push an image. Build inputs are pinned
and Cargo uses the committed lockfile. A future release still needs an explicit
artifact/version/verification decision. Historical `latest` images are not
immutable releases or evidence of a tested QualEvents deployment.

## Publication rules

- Do not publish crates, npm packages, containers, tags, releases, or deployments without explicit user instruction.
- A release ticket must identify artifact, version, target platforms, changelog, and verification commands.
- Publication credentials must never be committed.
- Release automation must not run on ordinary pull requests.
- Prefer dry runs before irreversible publication.

[#23](https://github.com/2001J/fuzzy-parser/issues/23) removes the earlier automatic
main-push image publication and its mismatch with this policy. Until that change
is integrated, branches still running the old workflow can publish on main
pushes; a local commit does not disable the remote workflow. No historical image
is deleted, and no release pipeline or publication permission is introduced.

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

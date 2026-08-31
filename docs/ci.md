# Continuous Integration

CI [automated checks after code changes] uses the existing
[workflow](../.github/workflows/ci.yml) for pull requests, pushes to `main`, and
`development`, and manual runs. It never publishes or deploys. Feature-branch
pushes without a pull request do not trigger duplicate runs.

## Required checks

| Check | What it verifies |
| --- | --- |
| Formatting, lint, version, and workflow | Rust formatting, warnings-as-errors Clippy across all targets, actionlint including embedded shell checks, and one aligned Rust/npm/WASM package version |
| Rust and Node | Locked workspace tests/builds on Ubuntu 24.04 x64 and macOS 15 arm64; fresh release CLI; CI guard tests; the existing Node 22 invocation/parity evaluation |
| WASM libraries (compilation only) | Application API, core, schema and formats libraries compile for `wasm32-unknown-unknown` |
| Installable Node/WASM package | Build/typecheck/test the selected package, pack/install it without consumer Rust tooling, and build/invoke a generic Next.js standalone fixture with Worker/WASM assets |
| Dependency advisories | Committed `Cargo.lock` against the current RustSec database using cargo-audit 0.22.2; advisory warnings and vulnerabilities fail the job |
| Batch CLI container (no publication) | Build/load Linux amd64 image; non-root execution; synthetic TXT/CSV/XLSX/stdin parsing, source evidence, review warnings, structured errors, usage exit codes and forced timeout cleanup |
| CI passed | Every required job, including both matrix legs, succeeded; failed, cancelled, skipped or missing jobs cannot pass |

Rust tests include unit, CLI subprocess and serialization/source-review
regressions. See [testing strategy](testing-strategy.md) for the dedicated layout.
The Node evaluation checks two supported synthetic profiles, exact CLI stream
parity, source resolution and bounded invocation controls; its
[scope and limitations](evaluations/2026-08-28-node-cli.md) still apply.

Reusable-library WASM compilation alone is not JavaScript execution. The
separate package job executes the selected Node adapter and verifies local
installation/framework packaging; it is still not publication or deployment.
Green CI does not establish full
[#19 independence](https://github.com/2001J/fuzzy-parser/issues/19), arbitrary
untrusted-file resource safety, benchmark targets or QualEvents integration.
The advisory job covers Rust dependencies, not operating-system image packages.

## Safety and maintenance

- Only `contents: read` permission; no persisted checkout credentials, registry
  login, publication, release or deployment step.
- Synthetic fixtures, no application secrets or production services. Tool,
  dependency and advisory downloads need network access. Parser containers run
  with no network, read-only filesystems/fixtures, no extra capabilities, a
  non-root UID, and memory/process limits.
- Every job has a timeout; obsolete runs are cancelled. Both matrix results stay
  visible. Cargo/image caches are saved only on `development` or `main` pushes; a
  cache hit never skips tests. Image cache export occurs during the build step,
  so a later semantic-smoke failure can still leave that cache available.
- Actions use upstream commit SHAs, actionlint and build images use digests,
  and the cargo-audit download is checked against a recorded SHA-256, following
  [GitHub's action security guidance](https://docs.github.com/en/actions/reference/security/secure-use).
- Rust **1.96.0** is a reproducible tested baseline, not a minimum-supported-Rust
  promise. Node **22** follows its available patch release. Update tool/action
  pins and image digests deliberately with relevant checks; never regenerate
  `Cargo.lock` during CI to bypass `--locked`. Hosted runner software and
  Buildx/BuildKit still follow runner/action defaults; not every environment
  component is frozen by these pins.
- Do not add blanket advisory ignores or `continue-on-error` to get green CI.
  Investigate failures; dependency fixes are separate reviewed changes.
- Keep `gate.needs` aligned with the explicit list in
  [check-results.mjs](../tools/ci/check-results.mjs). Permanent tests reject
  incomplete/non-successful job results and broken container parse semantics.

## Local reproduction

From the repository root with Rust 1.96.0 and Node 22 available:

```bash
tools/ci/verify-local.sh quick
tools/ci/verify-local.sh full
```

`quick` runs the locked Rust formatting, lint, test and workspace-build checks,
plus the deterministic installable Node-package verifier.
`full` adds the reusable-library WASM compilation check, release CLI, Node guard
tests and native invocation/parity evaluation. The script never installs a
toolchain or target, runs Docker, mutates Git, publishes or deploys. Set
`FP_VERIFY_EXPECTED_RUST` only when deliberately verifying with a different
already-active Rust version; the script does not select or download one.

The equivalent individual commands are:

```bash
cargo +1.96.0 fmt --check
cargo +1.96.0 clippy --workspace --all-targets --locked -- -D warnings
cargo +1.96.0 test --workspace --locked
cargo +1.96.0 build --workspace --locked
node --test tools/release/tests/*.test.mjs
node tools/release/check-version.mjs
node tools/ci/verify-node-package.mjs
cargo +1.96.0 build --release --locked -p parser-cli
node --test tools/ci/tests/*.test.mjs
node --check tools/runtime-evaluation/evaluate.mjs
node tools/runtime-evaluation/evaluate.mjs target/release/parser-cli
rustup target add --toolchain 1.96.0 wasm32-unknown-unknown
cargo +1.96.0 check --locked --target wasm32-unknown-unknown -p parser-api -p parser-core -p parser-schema -p parser-formats
```

Other checks require Docker, Cargo and cargo-audit **0.22.2**. The workflow records
a verified static Linux download; other platform binaries come from the
[same release](https://github.com/rustsec/rustsec/releases/tag/cargo-audit/v0.22.2).
Verify the archive SHA-256 before executing a downloaded tool.

```bash
cargo audit --file Cargo.lock --deny warnings
docker run --rm --network none --read-only --cap-drop ALL --security-opt no-new-privileges --mount "type=bind,source=$PWD,target=/repo,readonly" --workdir /repo rhysd/actionlint:1.7.12@sha256:b1934ee5f1c509618f2508e6eb47ee0d3520686341fec936f3b79331f9315667 -color
docker build --platform linux/amd64 --tag fuzzy-parser:ci .
node tools/ci/check-container.mjs fuzzy-parser:ci
```

The smoke check uses the already-built local image by ID; it never pulls or
pushes it. Normal test containers use `--rm`. A timeout kills the Docker client
with `SIGKILL`, then attempts forced removal of only its uniquely named
container with a separate timeout. If removal cannot be confirmed, the check
fails and reports that name for inspection; it does not claim successful cleanup.
A permanent smoke case verifies removal of a SIGTERM-resistant container.
Local images/build caches remain for inspection and reuse. No command publishes
an artifact.

## Hosted verification and merge protection

[#23](https://github.com/2001J/fuzzy-parser/issues/23) tracks implementation and
evidence. The first GitHub-hosted run of this revision remains outstanding
until an authorized push/pull request makes it available. Local macOS checks or
Linux containers cannot certify GitHub checkout, permissions, caches or runner
orchestration. Record the actual run URL and both platform results there.

After a green hosted run, `CI passed` is the intended stable required status
check for a repository ruleset or branch protection. This change does not alter
those settings: without required-check rules CI reports failures but does not
prevent merging. Reconcile any old required-check names when enabling the gate.

The manual [release workflow](../.github/workflows/release.yml) is separate from
CI. Its default execution validates and uploads candidate artifacts only. It can
publish only when an operator selects an explicit publication input on `main`;
ordinary `development`, `main`, pull-request and CI runs have read-only content
permissions and no registry credentials.

The old main-push workflow published a container independently of its Rust test
job. This checkout removes that behavior, but branches using the old workflow
retain it until integration. Historical images are not deleted. See
[publication rules](release-and-environment-strategy.md#publication-rules).

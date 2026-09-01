# Release Operator Guide

Fuzzy Parser uses one implementation version across the Rust workspace, CLI,
Node/WASM package and batch container. The parse-response, schema and error
contract versions are separate compatibility axes.

No release currently exists. Version `0.1.0` is the first coordinated candidate
line and remains unpublished until an operator deliberately completes this
guide.

## Normal development flow

```text
feature branch
-> pull request or reviewed integration into development
-> full CI on development
-> deliberate development-to-main pull request
-> full CI on main
-> manual Release workflow
```

Ordinary CI has read-only repository permission and cannot publish anything.

## Version change

Update the implementation version in:

- `[workspace.package].version` in `Cargo.toml`;
- `packages/fuzzy-parser-node/package.json`;
- `packages/fuzzy-parser-node/wasm/Cargo.toml`.

Then regenerate `Cargo.lock` deliberately and run:

```bash
node --test tools/release/tests/*.test.mjs
node tools/release/check-version.mjs 0.1.0
tools/ci/verify-local.sh full
```

The Node runtime and generated identity read the package version rather than
duplicating another source literal. The check also validates every workspace
package entry in `Cargo.lock`.

Use semantic versioning for implementation artifacts:

- patch for compatible fixes;
- minor for a new capability or intentional pre-1.0 compatibility change;
- major for stable-contract breaks after 1.0.

Document migrations whenever an existing JSON or typed consumer must change,
even before 1.0.

## Candidate artifacts without publication

From `development` or `main`, manually run `.github/workflows/release.yml` with
the exact version and leave every `publish_*` input false. The workflow:

1. verifies version alignment and the local quick profile;
2. builds Linux x64 and macOS arm64 CLI archives with SHA-256 files;
3. builds, tests, installs and packs `@fuzzy-parser/node` with a SHA-256 file;
4. stores them only as GitHub Actions run artifacts.

Each checksum file names only its adjacent archive basename, so downloaded
archive/checksum pairs can be verified from any directory with
`shasum -a 256 --check <archive>.sha256`.

This is reversible and does not create a tag, GitHub Release, npm version or
container image.

## Publication

Publication is allowed only from `main` after its CI run is green. Manually run
the Release workflow and select only the intended outputs:

- `publish_github`: creates immutable `v<version>` tag plus a GitHub Release
  containing CLI and npm archives/checksums;
- `publish_npm`: publishes the already-tested tarball with npm provenance and
  requires the protected `NPM_TOKEN` secret;
- `publish_container`: publishes immutable version and commit tags to GHCR.

All publication jobs use the GitHub `release` environment. Configure required
reviewers there before the first public release. Never select a publication
input merely to test the workflow; use the candidate mode above.

Rust crates are not published by this workflow. Add crate metadata and pass
`cargo publish --dry-run` in a separately reviewed release slice before enabling
that artifact.

## Release record and rollback

Record:

- implementation version and commit;
- parse, schema and error contract versions;
- supported inputs and platform artifacts;
- checksums, known limitations and migration notes;
- CI and Release workflow run URLs.

Rollback means installing the previous immutable npm/tag/container artifact or
CLI checksum, never moving an existing version tag or rewriting a published
package.

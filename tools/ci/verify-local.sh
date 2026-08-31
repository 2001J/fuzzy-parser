#!/usr/bin/env bash

set -euo pipefail

usage() {
  echo "usage: tools/ci/verify-local.sh <quick|full>" >&2
  exit 2
}

if [[ $# -ne 1 ]]; then
  usage
fi

readonly FP_VERIFY_PROFILE="$1"
readonly FP_VERIFY_EXPECTED_RUST="${FP_VERIFY_EXPECTED_RUST:-1.96.0}"
readonly FP_VERIFY_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

case "$FP_VERIFY_PROFILE" in
  quick | full) ;;
  *) usage ;;
esac

run() {
  printf '+ '
  printf '%q ' "$@"
  printf '\n'
  "$@"
}

cd "$FP_VERIFY_ROOT"

readonly FP_VERIFY_ACTUAL_RUST="$(rustc --version | awk '{print $2}')"
if [[ "$FP_VERIFY_ACTUAL_RUST" != "$FP_VERIFY_EXPECTED_RUST" ]]; then
  echo "error: active rustc is $FP_VERIFY_ACTUAL_RUST; expected $FP_VERIFY_EXPECTED_RUST" >&2
  exit 2
fi

run cargo fmt --check
run cargo clippy --workspace --all-targets --locked -- -D warnings
run cargo test --workspace --locked
run cargo build --workspace --locked
run node tools/ci/verify-node-package.mjs

if [[ "$FP_VERIFY_PROFILE" == "full" ]]; then
  if ! rustup target list --installed | grep -Fxq wasm32-unknown-unknown; then
    echo "error: wasm32-unknown-unknown is not installed for the active toolchain" >&2
    exit 2
  fi
  run cargo check --locked --target wasm32-unknown-unknown \
    -p parser-core -p parser-schema -p parser-formats
  run cargo build --release --locked -p parser-cli
  run node --test tools/ci/tests/*.test.mjs
  run node --check tools/runtime-evaluation/evaluate.mjs
  run node tools/runtime-evaluation/evaluate.mjs target/release/parser-cli
fi

run git diff --check

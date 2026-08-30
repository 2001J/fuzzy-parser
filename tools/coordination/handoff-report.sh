#!/usr/bin/env bash

set -euo pipefail

usage() {
  echo "usage: tools/coordination/handoff-report.sh <expected-base-commit>" >&2
  exit 2
}

if [[ $# -ne 1 ]]; then
  usage
fi

readonly FP_HANDOFF_BASE="$1"
readonly FP_HANDOFF_ROOT="$(git rev-parse --show-toplevel)"

cd "$FP_HANDOFF_ROOT"

if ! git cat-file -e "${FP_HANDOFF_BASE}^{commit}" 2>/dev/null; then
  echo "error: expected base is not a local commit: ${FP_HANDOFF_BASE}" >&2
  exit 2
fi

readonly FP_HANDOFF_HEAD="$(git rev-parse HEAD)"
readonly FP_HANDOFF_BRANCH="$(git symbolic-ref --quiet --short HEAD || true)"

if ! git merge-base --is-ancestor "$FP_HANDOFF_BASE" "$FP_HANDOFF_HEAD"; then
  echo "error: expected base ${FP_HANDOFF_BASE} is not an ancestor of HEAD ${FP_HANDOFF_HEAD}" >&2
  exit 1
fi

if [[ "$FP_HANDOFF_BASE" == "$FP_HANDOFF_HEAD" ]]; then
  readonly FP_HANDOFF_RELATION="exact"
else
  readonly FP_HANDOFF_RELATION="ancestor"
fi

if [[ -n "$(git status --porcelain=v1)" ]]; then
  readonly FP_HANDOFF_CLEAN="false"
else
  readonly FP_HANDOFF_CLEAN="true"
fi

print_paths() {
  local title="$1"
  shift
  printf '\n[%s]\n' "$title"
  "$@"
}

printf 'repository_root=%s\n' "$FP_HANDOFF_ROOT"
printf 'branch=%s\n' "${FP_HANDOFF_BRANCH:-DETACHED}"
printf 'head=%s\n' "$FP_HANDOFF_HEAD"
printf 'expected_base=%s\n' "$FP_HANDOFF_BASE"
printf 'base_relation=%s\n' "$FP_HANDOFF_RELATION"
printf 'clean=%s\n' "$FP_HANDOFF_CLEAN"

print_paths "committed_since_base" git diff --name-status "${FP_HANDOFF_BASE}..HEAD"
print_paths "staged" git diff --cached --name-status
print_paths "unstaged" git diff --name-status
print_paths "untracked" git ls-files --others --exclude-standard


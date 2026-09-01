#!/usr/bin/env bash

set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: tools/release/write-checksum.sh <archive>" >&2
  exit 2
fi

readonly FP_CHECKSUM_ARCHIVE="$1"

if [[ ! -f "$FP_CHECKSUM_ARCHIVE" ]]; then
  echo "error: archive is not a regular file: $FP_CHECKSUM_ARCHIVE" >&2
  exit 1
fi

readonly FP_CHECKSUM_DIRECTORY="$(cd "$(dirname "$FP_CHECKSUM_ARCHIVE")" && pwd)"
readonly FP_CHECKSUM_BASENAME="$(basename "$FP_CHECKSUM_ARCHIVE")"

cd "$FP_CHECKSUM_DIRECTORY"
echo "writing SHA-256 checksum: $FP_CHECKSUM_BASENAME.sha256"
shasum -a 256 "$FP_CHECKSUM_BASENAME" > "$FP_CHECKSUM_BASENAME.sha256"

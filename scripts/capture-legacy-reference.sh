#!/bin/sh
# Captures legacy payback/total responses for Phase B (docs/RUST_MIGRATION_PLAN.md §11.3).
# Usage: JIZIFIN_PASSPHRASE=... [OUT_DIR=dir] scripts/capture-legacy-reference.sh [path/to/finance.db]
# Works on a temporary COPY of the database; the original is never opened for writing.
set -eu
: "${JIZIFIN_PASSPHRASE:?export JIZIFIN_PASSPHRASE for this command only; it is never stored}"
root=$(cd "$(dirname "$0")/.." && pwd)
source_db=${1:-$root/backend/finance.db}
out=${OUT_DIR:-$root/backend/tests/reference/legacy}
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
cp "$source_db" "$work/finance.db"
mkdir -p "$out"
docker run --rm -e JIZIFIN_PASSPHRASE \
  -v "$work/finance.db:/app/finance.db" -v "$out:/out" \
  -v "$root/scripts/capture_legacy_reference.py:/capture.py:ro" \
  --entrypoint python jizifin-backend:python-last /capture.py

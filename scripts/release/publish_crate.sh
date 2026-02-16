#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE' >&2
Usage:
  scripts/release/publish_crate.sh [--dry-run]
USAGE
}

dry_run=false
if [[ $# -gt 1 ]]; then
  usage
  exit 1
fi
if [[ $# -eq 1 ]]; then
  case "$1" in
    --dry-run)
      dry_run=true
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      usage
      exit 1
      ;;
  esac
fi

if [[ -z "${CARGO_REGISTRY_TOKEN:-}" ]]; then
  echo "ERROR: missing CARGO_REGISTRY_TOKEN in environment" >&2
  exit 1
fi

if [[ "$dry_run" == "true" ]]; then
  cargo publish --dry-run --locked
  exit 0
fi

publish_err="$(mktemp)"
trap 'rm -f "$publish_err"' EXIT

if cargo publish --locked 2>"$publish_err"; then
  exit 0
fi

if grep -q "already uploaded" "$publish_err"; then
  echo "crate already published; skipping"
  exit 0
fi

cat "$publish_err" >&2
exit 1

# 0008: Pull transactional writes with `--write-on-error`
Date: 2026-02-11
Status: Accepted

## Context

`envgen pull` previously wrote resolved variables whenever at least one value succeeded, even if pull also had failures. This produced partial `.env` files by default while still returning exit code 1, which made failure handling ambiguous and could leave users with silently incomplete output.

Recent user behavior highlighted this mismatch directly: required command failures still resulted in a partially written destination file.

The project needs clearer default semantics for file writes during failed pulls, while preserving an explicit opt-in path for best-effort workflows.

## Decision

`envgen pull` now uses transactional write gating by default.

- By default, do **not** write the destination file when write-blocking failures occur.
- Write-blocking failures are:
  - any command-source failure (required or optional variable), or
  - any required non-command failure.
- Add `--write-on-error` to override the write gate and write resolved variables anyway.
- Even with `--write-on-error`, pull still exits with code 1 when failures occurred.
- If writes are blocked, the destination file is left untouched (including when `--force` is set).
- Pull output includes an explicit no-write message when write-blocking failures prevent writing.

## Consequences

- Default behavior is safer: no partial `.env` files are written when pull has write-blocking failures.
- Existing workflows that depended on partial writes now need `--write-on-error`.
- Optional command failures now produce non-zero exit code and block writes by default, increasing strictness.
- CLI help, docs, and tests must stay aligned with this behavior.

## Alternatives considered

1. Keep writing partial files by default and rely on exit code.
   - Rejected: still violates all-or-none expectations for most users.
2. Block writes only for required failures.
   - Rejected: optional command failures can still indicate real source instability and should block by default in this model.
3. Block writes for failures with no override option.
   - Rejected: removes useful best-effort workflows where partial output is intentionally acceptable.

## References/links

- `/Users/stevemorin/c/envgen/src/main.rs`
- `/Users/stevemorin/c/envgen/src/commands/pull.rs`
- `/Users/stevemorin/c/envgen/tests/test_pull.rs`
- `/Users/stevemorin/c/envgen/README.md`
- `/Users/stevemorin/c/envgen/prd.md`

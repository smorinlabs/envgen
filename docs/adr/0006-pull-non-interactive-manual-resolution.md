# 0006: Pull command is non-interactive for manual sources
Date: 2026-02-06
Status: Superseded

Superseded by: `0007-restore-pull-interactive-manual-prompts.md`

## Context

`envgen pull` previously supported an interactive prompt path for `source: manual` variables. The project now introduces separate interactive workflows for schema authoring (`init` and planned `add`) and needs clear command boundaries.

Having interactive behavior in `pull` created ambiguity:

- Two different meanings of "interactive" in the CLI.
- Harder automation and CI behavior for `pull`.
- Inconsistent expectations for required manual variables.

## Decision

`envgen pull` is non-interactive.

- Remove `--interactive` from `pull`.
- Do not prompt for manual values during pull.
- For `source: manual`:
  - `required: true` => treat as failure, continue processing others, return exit code 1.
  - `required: false` => skip with warning.

Interactive UX is reserved for schema authoring flows (`init` and `add`), not value resolution in `pull`.

## Consequences

- `pull` behavior is deterministic and CI-friendly.
- Required manual values are explicitly enforced instead of silently skipped.
- Existing docs/examples/tests that used `pull --interactive` must be updated.
- Manual prompt helper code remains available for non-pull interactive workflows.

## Alternatives considered

1. Keep `pull --interactive` and add `add/init --interactive` too.
   - Rejected: overlaps two interaction models and keeps command intent ambiguous.
2. Keep skipping all manual values regardless of `required`.
   - Rejected: allows required configuration to be silently omitted.

## References/links

- `/Users/stevemorin/c/envgen/src/main.rs`
- `/Users/stevemorin/c/envgen/src/commands/pull.rs`
- `/Users/stevemorin/c/envgen/prd.md`
- `/Users/stevemorin/c/envgen/prd-interactive.md`

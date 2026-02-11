# 0007: Restore pull interactive manual prompts
Date: 2026-02-06
Status: Accepted

## Context

ADR 0006 changed `envgen pull` to be fully non-interactive for `source: manual` variables.
That removed `pull --interactive` and forced required manual variables to fail pull.

After re-review, this behavior did not match the intended workflow for this project:

- `pull` still needs an explicit interactive path for manual values.
- default non-interactive pulls should skip manual values instead of forcing required failures.
- docs/examples and generated command hints should continue to show the interactive pull path.

## Decision

Restore `pull --interactive` and manual prompt behavior.

- Re-add `-i, --interactive` to `envgen pull`.
- When interactive mode is enabled, prompt for `manual` values during pull.
- When interactive mode is not enabled, skip `manual` values with warning output.
- Keep this behavior aligned across CLI help, tests, README, PRD, and sample schema guidance.

This ADR supersedes ADR 0006.

## Consequences

- Users can continue using `pull` for manual value entry when needed.
- Non-interactive pull remains automation-friendly because manual variables are skipped by default.
- Required manual variables are no longer forced to fail in default non-interactive mode.
- Documentation and tests must stay synchronized with the restored behavior.

## Alternatives considered

1. Keep ADR 0006 behavior and only support manual entry in `init`/`add`.
   - Rejected: does not match expected pull workflow for manual sources.
2. Make pull always interactive for manual values.
   - Rejected: breaks non-interactive/automation use cases.

## References/links

- `/Users/stevemorin/c/envgen/src/main.rs`
- `/Users/stevemorin/c/envgen/src/commands/pull.rs`
- `/Users/stevemorin/c/envgen/tests/test_pull.rs`
- `/Users/stevemorin/c/envgen/README.md`
- `/Users/stevemorin/c/envgen/prd.md`

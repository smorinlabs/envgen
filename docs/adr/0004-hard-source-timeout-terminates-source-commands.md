# 0004: Hard `--source-timeout` terminates source commands

Date: 2026-02-05

Status: Accepted

## Context

`envgen pull` supports resolving variables by executing shell commands defined in schema sources.

The `--source-timeout <seconds>` flag is documented as a timeout *for each source command*. The existing implementation used `tokio::time::timeout(..., Command::output())`, which stops waiting after the deadline but does not reliably terminate the underlying process.

This creates a critical safety issue: timed-out commands can continue running after `envgen` returns and can still perform side effects (e.g., writing files, mutating remote state), which is surprising and unsafe.

## Decision

Treat `--source-timeout` as a **hard execution timeout**:

- If a source command exceeds the configured timeout, `envgen` **terminates** it.
- On Unix platforms, `envgen` runs each source command in its own process group and, on timeout, sends a kill signal to the entire group to prevent child processes from surviving.
- If termination fails (best-effort), `envgen` still reports a timeout, but the implementation should be conservative and attempt to prevent side effects.

Timed-out commands are treated as failures, consistent with existing error-handling behavior.

## Consequences

- Users get predictable, bounded behavior from `--source-timeout`.
- Long-running sources must increase `--source-timeout` or redesign commands to complete within the limit.
- Termination is best-effort across platforms and cannot prevent side effects from processes that intentionally detach/daemonize themselves.

## Alternatives considered

- **Timeout means “stop waiting” only**: rejected due to unsafe, surprising behavior and mismatch with flag wording.
- **Add a separate flag for “hard timeout”**: rejected to keep behavior aligned with existing docs and security expectations.

## References

- EG-BUG-001 — Timed-out source commands keep running (Severity: Critical).


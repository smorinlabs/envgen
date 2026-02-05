# 0003: Add `envgen readme` command

Date: 2026-02-05

Status: Accepted

## Context

`envgen` ships as a standalone CLI binary. Users sometimes want quick, offline access to the full usage documentation (examples, safety notes, schema format) without needing to browse GitHub or keep a separate docs file in their repo.

While `envgen --help` is useful, it is intentionally brief and cannot replace the full project README.

## Decision

Add a new CLI command:

- `envgen readme`

Behavior:

- Prints the embedded project `README.md` to stdout.
- Takes no arguments.

## Consequences

- Adds a new public CLI command surface area (`envgen readme`).
- Requires keeping the embedded README reasonably accurate for released binaries.

## Alternatives considered

- Rely on `envgen --help` only (too limited for examples and detailed guidance).
- Link users to the GitHub README (not available offline).

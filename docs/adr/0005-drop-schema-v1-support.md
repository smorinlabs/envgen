# 0005: Drop `schema_version: "1"` support

Date: 2026-02-05

Status: Accepted

## Context

`envgen` previously accepted two schema versions:

- `schema_version: "1"` — single source per variable
- `schema_version: "2"` — single source per variable, plus optional per-environment resolvers via `variables.*.resolvers`

The project no longer needs to support v1. Keeping v1 support increases the surface area of validation, fixtures/tests, and documentation, without providing value.

## Decision

Support only `schema_version: "2"` schemas:

- Structural validation (embedded JSON Schema) only accepts `schema_version: "2"`.
- Semantic validation rejects any schema version other than `"2"`.
- Documentation and fixtures/tests are updated to use `"2"`.

Migration for v1 schemas is straightforward: set `schema_version: "2"` and keep existing single-source variable definitions unchanged (v2 still supports them).

## Consequences

- **Breaking change**: schemas declaring `schema_version: "1"` fail validation until updated.
- The codebase becomes simpler (no branching on schema version for validation).
- Future schema evolution can proceed from a single supported baseline.

## Alternatives considered

- **Keep v1 indefinitely**: rejected due to ongoing maintenance cost.
- **Deprecate v1 with warnings**: rejected to keep the tool strict and avoid prolonged dual-support.
- **Auto-migrate v1 → v2**: rejected for now; the manual migration is a single-line change and avoids modifying user files implicitly.


# 0002: Add optional source/resolver documentation fields

Date: 2026-02-05

Status: Accepted

## Context

`envgen` schemas are intended to be self-documenting, but documentation is currently limited to
variable-level fields. Sources have no human-friendly name or link, and resolver-specific context
must be stuffed into a variable description when a variable uses multiple resolvers.

## Decision

Add optional documentation fields:

- `sources.*`: `label`, `url`, `description`
- `variables.*.resolvers[]`: `label`, `url`, `description`

All fields are optional and purely descriptive. URL values remain plain strings (no strict `uri`
format validation). No runtime behavior changes are introduced.

## Consequences

- Existing schemas remain valid; the change is additive.
- JSON Schema and Rust schema types include the new optional fields.
- Sample schema and README are updated to show usage.
- Documentation rendering can optionally prefer resolver-level docs over source-level docs in
  the future, but no CLI behavior changes are required for this decision.

## Alternatives considered

- Use `name` instead of `label` (rejected to avoid confusion with map keys).
- Use prefixed field names like `source_url` or `docs_url` (rejected as noisy).
- Enforce URL validation with `format: uri` (rejected to keep validation minimal).

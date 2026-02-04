# 0003: Runtime structural validation via jsonschema crate

Date: 2026-02-04

Status: Accepted

## Context

`envgen` validates schema files in two phases:

1. Structural validation against the embedded JSON Schema.
2. Semantic validation via the Rust validator (cross-field rules).

ADR 0002 implemented structural validation with an internal JSON Schema subset validator to avoid new dependencies. Over time, this created maintenance overhead and constrained the embedded schema to a supported keyword subset.

## Decision

Use the `jsonschema` crate (Draft 2020-12) for runtime structural validation:

- Parse YAML into `serde_json::Value`.
- Validate with `jsonschema` (Draft 2020-12).
- If structural validation passes, run the existing semantic validator (`validator.rs`).

## Consequences

- Structural validation now supports the full JSON Schema draft implemented by the crate, reducing drift and maintenance.
- `envgen` adds a new runtime dependency (`jsonschema`), and enables the `draft202012` feature.
- Error messages may differ from the previous internal validator.

## Alternatives considered

- Keep the internal validator (lower dependencies, but higher maintenance and reduced schema feature support).
- Shell out to external tools like `check-jsonschema` (adds runtime toolchain requirements).

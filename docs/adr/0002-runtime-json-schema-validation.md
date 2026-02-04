# 0002: Runtime structural validation via embedded JSON Schema

Date: 2026-02-04

Status: Accepted

## Context

`envgen` ships a versioned JSON Schema (`schemas/envgen.schema.vX.Y.Z.json`) and exposes it via `envgen schema`.
Historically, `envgen` commands validated schema YAML files via:

- `serde`/`serde_yaml` decoding into Rust types (including `deny_unknown_fields`)
- a custom semantic validator (`envgen check` rules) used by some commands

This created two problems:

1. The shipped JSON Schema was primarily for editor tooling and was not actually used by the CLI at runtime.
2. Validation behavior was inconsistent across commands (e.g., some commands parsed a schema but did not run the full validator).

## Decision

All commands that consume a schema file (`check`, `pull`, `list`, `docs`) run a shared validation pipeline:

1. Parse YAML to a generic value.
2. **Structural validation** against the embedded JSON Schema.
3. **Semantic validation** via the existing Rust validator.

If structural validation fails, semantic validation is skipped and only structural errors are reported.
There are no flags to bypass either layer; behavior is fixed and consistent (including `pull` vs `pull --dry-run`).

To avoid introducing a new dependency in restricted/offline environments, `envgen` implements an internal JSON Schema validator that supports the subset of Draft 2020-12 features used by the embedded schema.

## Consequences

- The embedded JSON Schema is now the runtime source of truth for structural constraints (required fields, unknown keys, naming patterns, min sizes, etc.).
- `envgen list` now fails on invalid schemas instead of attempting to operate on partially-invalid inputs.
- Structural error messages are reported using JSON Pointer-style paths (e.g., `/variables/FOO/source`).
- Changes to `schemas/envgen.schema.vX.Y.Z.json` must stay within the supported keyword subset (or the validator must be extended).

## Alternatives considered

- Use a third-party Rust JSON Schema validator crate (adds a dependency; not viable in offline environments).
- Shell out to external tools like `check-jsonschema` (adds runtime toolchain requirements and complicates distribution).
- Re-encode the structural rules directly in Rust (risk of drift from the shipped JSON Schema).


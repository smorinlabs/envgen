# 0010: Decouple crate and schema artifact versioning
Date: 2026-02-13
Status: Accepted

## Context

`envgen` previously tied the embedded schema artifact filename/version to the crate version via
`CARGO_PKG_VERSION`. That coupling forced schema artifact renames for every crate release, even
when schema content was unchanged.

This repository now needs independent release cadence for:

- crate/binary releases (`vX.Y.Z`), and
- schema artifact releases (`schema-vA.B.C`).

The release process also needs explicit safety boundaries so version prep does not implicitly
create tags or trigger publish automation.

## Decision

Adopt a `make` wrapper with a dedicated script (`scripts/version_bump.py`) and separate version
sources of truth.

- Crate stream:
  - source of truth: `[package].version` in `Cargo.toml`
  - changelog: `CHANGELOG.md`
  - tags: `vX.Y.Z`
- Schema stream:
  - source of truth: `SCHEMA_VERSION`
  - artifact file: `schemas/envgen.schema.vA.B.C.json`
  - changelog: `SCHEMA_CHANGELOG.md`
  - tags: `schema-vA.B.C`

Bump commands and tag commands are intentionally split:

- `make bump-*` updates files only and never tags.
- `make tag-*` creates local annotated tags only.
- `make push-tag-*` pushes existing tags only.

Tag commands are file-first and allow env-var overrides for explicitness:

- crate tags: `VERSION` override
- schema tags: `SCHEMA_VERSION` override

Overrides must match file-derived versions; mismatches fail fast.

Schema artifact version is injected at compile time using `build.rs` and
`ENVGEN_SCHEMA_ARTIFACT_VERSION`, replacing prior dependence on `CARGO_PKG_VERSION`.

## Consequences

- Crate and schema artifacts can ship independently without unnecessary schema renames.
- Release intent is clearer and safer because tagging/pushing is always explicit.
- The repo gains a small amount of custom script/Makefile logic to maintain.
- Documentation and tests now model two release streams and two changelogs.

## Alternatives considered

1. Keep crate and schema versions coupled.
   - Rejected: forces unnecessary schema artifact churn.
2. Adopt cargo-native release tooling as primary mechanism.
   - Rejected for now: still requires custom logic for schema artifacts, with added dependency
     overhead.
3. Automate tag creation during bump.
   - Rejected: increases accidental release trigger risk.

## References/links

- `/Users/stevemorin/c/envgen/Makefile`
- `/Users/stevemorin/c/envgen/scripts/version_bump.py`
- `/Users/stevemorin/c/envgen/SCHEMA_VERSION`
- `/Users/stevemorin/c/envgen/SCHEMA_CHANGELOG.md`
- `/Users/stevemorin/c/envgen/build.rs`
- `/Users/stevemorin/c/envgen/src/schema/mod.rs`
- `/Users/stevemorin/c/envgen/tests/test_schema.rs`
- `/Users/stevemorin/c/envgen/RELEASING.md`

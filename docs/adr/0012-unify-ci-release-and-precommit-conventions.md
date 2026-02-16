# 0012: Unify CI, release, and pre-commit quality gate conventions
Date: 2026-02-16
Status: Accepted

## Context

Quality checks and release validation drifted across multiple entrypoints:

- CI and release workflows used overlapping but non-identical command sets.
- Pre-commit hooks duplicated Rust checks directly and used mutating formatting commands.
- Tag/version validation and publish logic were duplicated between release and fallback workflows.
- Toolchain setup was not patch-pinned in every execution context.

This created avoidable maintenance cost, inconsistent behavior between local and CI/release, and
higher risk of release surprises.

## Decision

Adopt a single-source convention where the `Makefile` defines canonical quality gates, and all
automation layers call those targets.

Concretely:

- Pin toolchain policy to exact patch `1.88.0` with `rust-toolchain.toml`.
- Align `Cargo.toml` `rust-version` to `1.88.0` for exact parity.
- Define make targets by profile:
  - `check-core`
  - `check-msrv`
  - `check-security`
  - `check-release`
  - `precommit-fast`
  - `prepush-full`
- Update CI and release workflows to call shared make targets instead of duplicating raw commands.
- Replace duplicated workflow inline logic with shared scripts:
  - `scripts/release/validate_tag.py`
  - `scripts/release/publish_crate.sh`
- Convert pre-commit into a wrapper around make targets:
  - pre-commit stage runs `make precommit-fast`
  - pre-push stage runs `make prepush-full`

## Consequences

- CI and release Linux checks now run the same core gate implementation.
- Pre-commit behavior becomes non-mutating and consistent with CI/release semantics.
- Release and fallback workflows share one implementation for tag validation and publish idempotency.
- Toolchain/version expectations are explicit and deterministic across local and automation contexts.
- Workflow and Makefile changes increase up-front structure, but reduce long-term duplication.

## Alternatives considered

1. Keep each workflow/pre-commit path independent with ad-hoc updates.
   - Rejected: continues drift and duplicate maintenance.
2. Move all logic into workflow YAML only.
   - Rejected: makes local parity harder and weakens contributor ergonomics.
3. Hard-cut remove fallback workflow immediately.
   - Rejected: migration safety requirement keeps temporary fallback path in place.

## References/links

- `/Users/stevemorin/c/envgen/Makefile`
- `/Users/stevemorin/c/envgen/.github/workflows/ci.yml`
- `/Users/stevemorin/c/envgen/.github/workflows/release.yml`
- `/Users/stevemorin/c/envgen/.github/workflows/publish-fallback.yml`
- `/Users/stevemorin/c/envgen/.pre-commit-config.yaml`

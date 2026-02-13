# 0011: Raise MSRV to apply RustSec fix for `time`
Date: 2026-02-13
Status: Accepted

## Context

The pre-commit `cargo-audit` hook started failing on advisory `RUSTSEC-2026-0009`
(`CVE-2026-25727`, `GHSA-r6v5-fh4h-64xc`) affecting `time` versions `>=0.3.6, <0.3.47`.

This project pinned:

- `time = "=0.3.36"` in `Cargo.toml`
- `rust-version = "1.85"`

The patched `time` release (`0.3.47`) requires Rust `1.88.0`. Keeping the existing MSRV
prevents adoption of the security fix.

## Decision

Raise the project MSRV from `1.85` to `1.88` and pin `time` to `=0.3.47`.

Concretely:

- update `[package].rust-version` to `"1.88"`
- update dependency pin to `time = "=0.3.47"`
- refresh `Cargo.lock` so `time`, `time-core`, and `time-macros` resolve to patched versions

No `cargo-audit` advisory ignore is added. The vulnerability is remediated in dependencies.

## Consequences

- `cargo audit` can pass without exceptions for this advisory.
- Users/build systems pinned below Rust `1.88` can no longer build the crate.
- MSRV validation in CI remains aligned because workflow logic reads `rust-version` from
  `Cargo.toml`.
- The dependency graph remains stable with an explicit `time` pin, reducing unexpected upgrades.

## Alternatives considered

1. Upgrade `jsonschema` to a newer major/minor series that no longer depends on `time`.
   - Rejected for now: larger dependency/API migration surface than needed for immediate
     remediation.
2. Keep MSRV at `1.85` and suppress `RUSTSEC-2026-0009` in `cargo-audit`.
   - Rejected: leaves known vulnerability unresolved in the lockfile.
3. Unpin `time` and allow semver float.
   - Rejected: weaker dependency determinism than current project policy.

## References/links

- `/Users/stevemorin/c/envgen/Cargo.toml`
- `/Users/stevemorin/c/envgen/Cargo.lock`
- `/Users/stevemorin/c/envgen/.pre-commit-config.yaml`
- `https://rustsec.org/advisories/RUSTSEC-2026-0009.html`

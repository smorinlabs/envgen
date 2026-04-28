# ADR 0016: Raise MSRV to 1.91.1 for cargo-msrv 0.19

## Status

Accepted — 2026-04-27

## Context

`cargo-msrv` released 0.19.x (current 0.19.3) requiring rustc ≥ 1.91.1. The
project's MSRV was 1.88.0, set in ADR-0011 to accommodate the patched `time`
crate for RUSTSEC-2026-0009. Pinning `cargo-msrv@0.18.4` keeps the old toolchain
working but freezes the MSRV-check tooling on a release line that will not
receive bug fixes or feature updates.

## Decision

Raise the project MSRV from `1.88.0` to `1.91.1`. Update every place the
toolchain is pinned to keep local development, CI, and release automation in
lockstep:

- `rust-toolchain.toml` channel
- `Cargo.toml` `rust-version`
- `Makefile` `RUST_TOOLCHAIN_PIN`
- All three GitHub Actions workflows (`ci.yml`, `release.yml`, `publish-fallback.yml`)
- `RELEASING.md` toolchain-parity policy

Pin `cargo-msrv@0.19.3` explicitly in `make install-cargo-msrv` for build
determinism (matches the pinned-version convention already used for `typos-cli`).

## Consequences

- MSRV-check tooling stays on the supported release line.
- Users/build systems pinned below Rust `1.91.1` can no longer build the crate.
  Acceptable: 1.91.1 has been stable since shortly before this ADR, and the
  project's release cadence is fast enough that consumers track latest stable.
- The `time = "=0.3.47"` pin and its RUSTSEC-2026-0009 rationale are unchanged;
  the comment was updated to read "1.88+" for accuracy.

## Alternatives considered

- **Stay on `cargo-msrv@0.18.4`.** Works today but accumulates tooling debt
  with each release skipped.
- **Drop `cargo-msrv` from CI entirely.** MSRV verification is load-bearing —
  the project ships to crates.io and downstream consumers respect `rust-version`.

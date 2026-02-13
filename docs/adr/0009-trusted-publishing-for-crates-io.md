# 0009: Trusted Publishing for crates.io
Date: 2026-02-12
Status: Accepted

## Context

`envgen` publishing was previously authenticated in GitHub Actions with a long-lived
`CARGO_REGISTRY_TOKEN` secret. That model works but increases secret management burden and
blast radius if a token is leaked.

As of 2025, crates.io supports Trusted Publishing with GitHub OIDC, allowing workflows to obtain
short-lived publish tokens without storing a long-lived registry token. This is now the preferred
security model for GitHub Actions-based crate publishing.

The release workflow should move to Trusted Publishing with minimal release interruption risk.

## Decision

Adopt a phased migration to crates.io Trusted Publishing.

- Make Trusted Publishing the default auth path in `.github/workflows/release.yml`.
- Authenticate in the publish job via `rust-lang/crates-io-auth-action` and use the temporary
  token for `cargo publish`.
- Keep publish idempotency behavior (`already uploaded` => no-op success).
- Add workflow hardening in release automation:
  - least-privilege per-job permissions,
  - pinned GitHub Action commit SHAs,
  - workflow concurrency control to reduce duplicate release races.
- Keep a temporary manual fallback workflow (`.github/workflows/publish-fallback.yml`) that uses
  `CARGO_REGISTRY_TOKEN` for emergency publish retries only.
- Retire fallback after 2 successful Trusted Publishing releases.

Additionally, fix package determinism by root-anchoring `Cargo.toml` `include` entries and adding
a CI guard that fails if local cache/tool directories appear in packaged output.

## Consequences

- Security posture improves by removing long-lived token dependency from the main release path.
- Release operations become more robust against duplicate-trigger races.
- Workflow files become more explicit and slightly more complex due to SHA pinning and
  per-job permission declarations.
- A temporary operational burden remains while fallback is active; this is intentionally time-boxed.
- Packaging behavior is more deterministic across developer and CI environments.

## Alternatives considered

1. Keep token-based publishing and only harden permissions.
   - Rejected: does not address long-lived token risk.
2. Immediate full cutover without fallback.
   - Rejected: higher migration risk if trusted publisher mapping is misconfigured at cutover time.
3. Continue with local/manual publishing outside Actions.
   - Rejected: reduces automation consistency and traceability.

## References/links

- https://blog.rust-lang.org/2025/07/11/crates-io-trusted-publishing/
- https://crates.io/docs/trusted-publishing
- https://github.com/rust-lang/crates-io-auth-action
- https://docs.github.com/en/actions/security-for-github-actions/security-hardening-your-deployments/configuring-openid-connect-in-cloud-providers
- https://docs.github.com/en/actions/reference/security/secure-use
- https://doc.rust-lang.org/cargo/reference/manifest.html#the-exclude-and-include-fields
- `/Users/stevemorin/c/envgen/.github/workflows/ci.yml`
- `/Users/stevemorin/c/envgen/.github/workflows/release.yml`
- `/Users/stevemorin/c/envgen/.github/workflows/publish-fallback.yml`
- `/Users/stevemorin/c/envgen/Cargo.toml`
- `/Users/stevemorin/c/envgen/RELEASING.md`

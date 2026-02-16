# 0013: Separate Homebrew tap release flow from core crate release pipeline
Date: 2026-02-16
Status: Accepted

## Context

`envgen` now has a stable release line (`v1.0.0`) and needs Homebrew distribution that is available immediately,
independent from `homebrew-core` acceptance and cadence.

The existing release workflow in this repository already handles crates.io publishing, GitHub release notes,
and binary asset uploads. Homebrew distribution requires additional responsibilities:

- resolving the GitHub source tarball URL and SHA256 per tag
- managing formula updates in a dedicated tap repository
- opening and updating pull requests against that tap

Coupling those responsibilities directly into `homebrew-core` automation would make `homebrew-core` state a release
risk, while the project still needs a dependable personal tap path.

## Decision

Adopt a two-repo Homebrew strategy:

- Keep `smorinlabs/envgen` as the automation source-of-truth and release producer.
- Host formulae in `smorinlabs/homebrew-tap` as the primary Homebrew distribution channel.
- Add a dedicated workflow (`.github/workflows/homebrew-tap-pr.yml`) to open/update tap PRs when releases are published.
- Add Python automation (`scripts/homebrew/tap_release.py`) and `make homebrew-*` commands with guided next-step hints.
- Keep the existing `homebrew-core` bump job non-blocking and future-facing.

## Consequences

- Homebrew tap updates are isolated from crate publishing and release asset generation.
- Release reliability improves because tap and core integration failures do not block crate releases.
- Formula update automation becomes deterministic and repeatable across local and CI contexts.
- A dedicated token (`HOMEBREW_TAP_GITHUB_TOKEN`) is required for cross-repo PR automation.
- There is one additional repository (`smorinlabs/homebrew-tap`) to maintain.

## Alternatives considered

1. Monorepo tap strategy (`smorinlabs/envgen` as tap host).
   - Rejected: weaker isolation, higher permission blast radius, and less conventional tap UX.
2. `homebrew-core` only.
   - Rejected: introduces external acceptance/cadence as a release dependency.
3. Manual tap updates only.
   - Rejected: too error-prone and inconsistent for repeatable releases.

## References/links

- `/Users/stevemorin/c/envgen/.github/workflows/release.yml`
- `/Users/stevemorin/c/envgen/.github/workflows/homebrew-tap-pr.yml`
- `/Users/stevemorin/c/envgen/scripts/homebrew/tap_release.py`
- `/Users/stevemorin/c/envgen/Makefile`
- `/Users/stevemorin/c/envgen/RELEASING.md`

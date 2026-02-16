# 0014: Run Homebrew tap PR workflow on macOS
Date: 2026-02-16
Status: Accepted

## Context

The Homebrew tap workflow (`.github/workflows/homebrew-tap-pr.yml`) runs the
`homebrew-verify-formula` step before opening or updating a tap PR.

That verification path executes `brew style`, `brew audit`, `brew install`, and
`brew test` via `scripts/homebrew/tap_release.py`.

The job previously ran on `ubuntu-latest`, where `brew` is not available by
default. This caused the workflow to fail before PR creation with:

- `ERROR: \`brew\` is required for formula verification`

As a result, release-to-tap automation could not complete even when tags and
token permissions were valid.

## Decision

Run the `update-tap` job in `.github/workflows/homebrew-tap-pr.yml` on
`macos-latest` instead of `ubuntu-latest`.

## Consequences

- Formula verification now executes in an environment where Homebrew is
  available by default, allowing the workflow to proceed through verification
  and PR creation.
- The workflow keeps full verification coverage (`style`, `audit`, `install`,
  `test`) instead of weakening checks.
- Runtime/cost may increase relative to Ubuntu-hosted execution.

## Alternatives considered

1. Install Linuxbrew in the Ubuntu job.
   - Rejected: adds setup complexity and more moving parts in a release
     automation path.
2. Skip formula verification when `brew` is missing.
   - Rejected: reduces safety by allowing unverified formula changes.

## References/links

- `/Users/stevemorin/c/envgen/.github/workflows/homebrew-tap-pr.yml`
- `https://github.com/smorinlabs/envgen/actions/runs/22078918496`

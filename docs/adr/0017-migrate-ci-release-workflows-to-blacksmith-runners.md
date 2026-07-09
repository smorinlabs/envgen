# 0017: Migrate CI and release workflows to Blacksmith runners
Date: 2026-07-09
Status: Accepted

## Context

CI, PR validation, and release automation ran on GitHub-hosted runners
(`ubuntu-latest`, `macos-latest`). GitHub-hosted runners are comparatively slow
and expensive, provide no colocated Actions cache, and offer limited
observability into job execution.

Blacksmith provides drop-in replacement runners that boot from the same images,
run on faster hardware at lower per-minute cost, transparently back the official
GitHub cache actions with a colocated cache, and add run-level logs and metrics.
Switching is a `runs-on:` label change, so this is a CI/release strategy change
under the ADR rule for "Build/release/CI strategy changes."

## Decision

Move the selected Linux and macOS jobs to Blacksmith runner labels while keeping
every other step, permission, and release behavior unchanged:

- Linux jobs: `ubuntu-latest` -> `blacksmith-4vcpu-ubuntu-2404` (CI core/MSRV/
  security checks, conventional-commits validation, and the release
  metadata/publish/GitHub-release/dispatch jobs).
- macOS jobs: `macos-latest` -> `blacksmith-6vcpu-macos-latest` (the CI Rust
  macOS check, the Homebrew tap PR workflow, and the release homebrew-core bump
  job). The `-macos-latest` label is used rather than a pinned `-macos-15` so
  these jobs keep tracking the latest macOS image, preserving the semantics of
  the original `macos-latest` labels.
- Keep `publish-fallback.yml` on GitHub-hosted `ubuntu-latest`, not Blacksmith.
  `RELEASING.md` documents it as the temporary emergency fallback for release
  safety, so it must stay an independent recovery path: if Blacksmith is
  unavailable or misconfigured, the token fallback must not fail together with
  the Blacksmith-hosted trusted-publishing path.
- The `release.yml` `verify`/`build` matrices are not migrated to Blacksmith and
  stay GitHub-hosted, but their Ubuntu entries are pinned from `ubuntu-latest` to
  `ubuntu-24.04` so the release path tests the same Ubuntu image as the pinned
  Blacksmith PR check (`blacksmith-4vcpu-ubuntu-2404`), preventing Ubuntu version
  drift between PR CI and tag releases. Their `macos-latest` entries are left
  unchanged, matching the CI macOS check that also tracks latest.
- Register the custom Blacksmith labels in `.github/actionlint.yaml` so
  `make lint-actions` (actionlint) does not reject them as unknown runner labels.

## Consequences

- CI and release jobs run on faster, cheaper hardware with colocated caching and
  richer observability, with no change to the checks or release logic they run.
- macOS jobs continue to track the latest macOS image, so PR CI and the release
  path stay aligned on macOS version rather than diverging via a pinned tag.
- Blacksmith `runs-on:` labels require the Blacksmith GitHub App to be installed
  on the repository; the actionlint config must list any new Blacksmith labels
  introduced later.
- The token publish fallback retains an infrastructure-independent recovery path,
  so a Blacksmith outage cannot take out both crates.io publish routes at once.
- Release Ubuntu builds are now pinned to `ubuntu-24.04` instead of tracking
  `ubuntu-latest`, so they will not silently move to a newer image (e.g. when
  `ubuntu-latest` advances) ahead of the PR check that gates them.

## Alternatives considered

1. Pin macOS jobs to `blacksmith-6vcpu-macos-15`.
   - Rejected: silently changes the original `macos-latest` semantics and lets
     the PR macOS check drift from the release matrices' macOS version.
2. Keep all workflows on GitHub-hosted runners.
   - Rejected: forgoes the speed, cost, cache, and observability benefits with no
     offsetting advantage.
3. Migrate the release `verify`/`build` matrices to Blacksmith in the same change.
   - Deferred: out of scope for this runner-only migration; can be revisited
     separately. Only their Ubuntu image is pinned to `ubuntu-24.04` for parity.
4. Migrate `publish-fallback.yml` to Blacksmith alongside the primary path.
   - Rejected: it is the emergency recovery path and must stay independent of
     Blacksmith infrastructure.

## References/links

- `.github/actionlint.yaml`
- `.github/workflows/ci.yml`
- `.github/workflows/conventional-commits.yml`
- `.github/workflows/homebrew-tap-pr.yml`
- `.github/workflows/publish-fallback.yml`
- `.github/workflows/release.yml`
- `RELEASING.md` (Emergency fallback section)
- https://docs.blacksmith.sh/blacksmith-runners/overview#runner-tags

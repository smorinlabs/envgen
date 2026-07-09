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
  security checks, conventional-commits validation, release metadata/publish/
  GitHub-release/dispatch jobs, and the crates.io publish fallback).
- macOS jobs: `macos-latest` -> `blacksmith-6vcpu-macos-latest` (the CI Rust
  macOS check, the Homebrew tap PR workflow, and the release homebrew-core bump
  job). The `-macos-latest` label is used rather than a pinned `-macos-15` so
  these jobs keep tracking the latest macOS image, preserving the semantics of
  the original `macos-latest` labels.
- The `release.yml` `verify`/`build` matrices remain on GitHub-hosted
  `macos-latest`; they were intentionally left out of this migration's scope.
- Register the custom Blacksmith labels in `.github/actionlint.yaml` so
  `make lint-actions` (actionlint) does not reject them as unknown runner labels.

## Consequences

- CI and release jobs run on faster, cheaper hardware with colocated caching and
  richer observability, with no change to the checks or release logic they run.
- macOS jobs continue to track the latest macOS image, so PR CI and the release
  path stay aligned on macOS version rather than diverging via a pinned tag.
- `runs-on:` labels are now Blacksmith-specific and require the Blacksmith GitHub
  App to be installed on the repository; the actionlint config must list any new
  Blacksmith labels introduced later.
- The GitHub-hosted `verify`/`build` release matrices are unchanged, so their
  cost/performance profile is unaffected by this ADR.

## Alternatives considered

1. Pin macOS jobs to `blacksmith-6vcpu-macos-15`.
   - Rejected: silently changes the original `macos-latest` semantics and lets
     the PR macOS check drift from the release matrices' macOS version.
2. Keep all workflows on GitHub-hosted runners.
   - Rejected: forgoes the speed, cost, cache, and observability benefits with no
     offsetting advantage.
3. Migrate the release `verify`/`build` matrices in the same change.
   - Deferred: out of scope for this runner-only migration; can be revisited
     separately.

## References/links

- `.github/actionlint.yaml`
- `.github/workflows/ci.yml`
- `.github/workflows/conventional-commits.yml`
- `.github/workflows/homebrew-tap-pr.yml`
- `.github/workflows/publish-fallback.yml`
- `.github/workflows/release.yml`
- https://docs.blacksmith.sh/blacksmith-runners/overview#runner-tags

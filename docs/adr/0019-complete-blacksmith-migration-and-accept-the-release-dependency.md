# 0019: Complete the Blacksmith migration and accept the release dependency
Date: 2026-08-16
Status: Accepted

Extends ADR-0018, which migrated CI and part of the release workflow to
Blacksmith runners. ADR-0018 remains in force; this record moves the remaining
release jobs and states the resulting failure mode explicitly.

## Context

ADR-0018 left three things on GitHub-hosted runners: the `release.yml` `verify`
matrix, the `release.yml` `build` matrix, and `publish-fallback.yml`. That split
was not a deliberate boundary — `verify` and `build` were simply not part of the
migration wizard's output, while `publish-fallback.yml` was held back on purpose
as an independent recovery path.

Reviewing the release job graph made the actual dependency structure visible:

```text
meta (Blacksmith, 5s)          <- every release job needs this
├── verify (GitHub-hosted, 109s / 130s)
│   ├── publish-crates (Blacksmith, 98s)   -> covered by publish-fallback.yml
│   │   └── homebrew-core (Blacksmith, 2s)
│   └── github-release (Blacksmith, 8s)    -> no fallback
│       ├── dispatch-homebrew-tap-pr (Blacksmith, 4s)
│       └── build (GitHub-hosted, 88s / 100s / 197s)
```

Durations are measured from the v1.0.6 release run (2026-02-16).

Two facts follow. First, the mixed hosting bought no resilience: `meta` is
Blacksmith-hosted and every other job declares `needs: meta`, so a Blacksmith
outage already stalls the entire pipeline — including the GitHub-hosted jobs,
which never become eligible to start. Second, `build` is GitHub-hosted yet
declares `needs: github-release`, so it is stranded behind a Blacksmith job even
though its own runner is unaffected.

The split therefore carried the cost of running release verification and binary
builds on slower hardware without buying the independence it appeared to offer.

## Decision

Move the remaining `release.yml` jobs to Blacksmith and state the consequence
plainly rather than leaving a partial mitigation that does not hold.

- Move the `verify` and `build` matrices to Blacksmith runners.
- Introduce a `runner` key in both matrices and point `runs-on` at
  `matrix.runner`. `matrix.os` keeps its existing identity role: it names the
  check (`Verify (ubuntu-24.04)`) and gates ten `if: matrix.os == ...` step
  conditionals. Changing the `os` values instead would have required rewriting
  every one of those conditionals, where a single missed edit silently skips a
  release step.
- Use `os: windows-2025` in the `build` matrix rather than `windows-latest`.
  Blacksmith publishes no `-windows-latest` alias, only versioned Windows
  images, so the identity key is pinned to match the runner instead of claiming
  to track latest. This mirrors the Ubuntu pinning ADR-0018 applied for the same
  reason.
- Register `blacksmith-4vcpu-windows-2025` in `.github/actionlint.yaml`.
- **Keep `publish-fallback.yml` on GitHub-hosted `ubuntu-latest`.** This is the
  one deliberate exception and it is not an oversight. The workflow is
  `workflow_dispatch`-only, runs by hand in an emergency, and does no
  compilation, so faster hardware gains it nothing. Its entire purpose is to
  survive a Blacksmith outage; moving it would delete the last independent
  recovery path in exchange for no measurable benefit.

## Consequences

- **Blacksmith is a single point of failure for the release pipeline.** A
  single point of failure is one component whose unavailability stops everything
  downstream. If Blacksmith is unavailable, misconfigured, or its GitHub App is
  uninstalled, no `release.yml` job can run — `meta` cannot start, and every
  other job waits on it.
- This is a statement of the existing condition, not a new exposure. `meta` was
  already Blacksmith-hosted under ADR-0018, so the pipeline already stopped
  entirely during an outage. What changes is that the mixed hosting no longer
  implies a resilience it never provided.
- **What survives an outage:** publishing to crates.io, by dispatching
  `publish-fallback.yml` on a GitHub-hosted runner.
- **What does not survive an outage:** creating the GitHub release, building and
  uploading the platform binaries, opening the Homebrew tap pull request, and
  opening the homebrew-core bump pull request. These have no independent path
  and must wait for Blacksmith to return, or be performed manually.
- `RELEASING.md` carries the recovery runbook for that state.
- Release verification and binary builds run on faster hardware with colocated
  caching, matching the rest of CI.
- Release asset filenames embed `${{ runner.os }}-${{ runner.arch }}`, currently
  producing `envgen-<tag>-Linux-X64.tar.gz`, `-macOS-ARM64.tar.gz`, and
  `-Windows-X64.zip`. The Blacksmith images are expected to report identical
  values, since the Ubuntu label is the non-`-arm` variant, the macOS runners are
  Apple Silicon, and the Windows image is x64. This has not been verified on a
  real run — see the verification gap below.

## Verification gap

`release.yml` runs only on a `v*.*.*` tag push or a manual dispatch, so no pull
request can exercise these jobs. Dispatching it against an existing tag is not a
safe test: it would re-run `publish-crates` and `github-release` against an
already-published version.

Two of the three runner labels are already proven in this repository —
`blacksmith-4vcpu-ubuntu-2404` and `blacksmith-6vcpu-macos-latest` run every CI
job today. `blacksmith-4vcpu-windows-2025` has never run here and would debut on
the release path, as would the `runner.arch` values above.

Exercising the Windows label once in `ci.yml` before the next release is the
cheap way to close this. It is not done in this change.

## Alternatives considered

- **Move `meta` and `github-release` back to GitHub-hosted instead.** Thirteen
  seconds of work, and it would have made the core release path independent of
  Blacksmith while leaving `publish-crates` on the faster runner. Rejected: it
  optimizes for an outage rare enough that waiting and re-running the tag is an
  acceptable response, and it leaves the repository with a hosting split that
  has to be explained and maintained.
- **Move everything including `publish-fallback.yml`.** Maximum uniformity and
  the simplest description. Rejected: it removes the only recovery path in the
  repository, reversing a decision ADR-0018 made deliberately in response to
  review, in exchange for no speed benefit on a workflow that runs by hand.
- **Document the failure mode and change no runners.** Delivers the clarity
  without the migration. Rejected: it leaves release verification and binary
  builds on slower hardware for a resilience benefit that the `needs: meta`
  edge already nullifies.

## References/links

- `.github/workflows/release.yml`
- `.github/workflows/publish-fallback.yml`
- `.github/actionlint.yaml`
- `RELEASING.md`
- `docs/adr/0018-migrate-ci-release-workflows-to-blacksmith-runners.md`
- Blacksmith runner tags: https://docs.blacksmith.sh/blacksmith-runners/overview

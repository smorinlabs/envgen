# Releasing `envgen`

This repo has two independent release streams:

- Crate release stream:
  - Version source: `Cargo.toml` (`[package].version`)
  - Changelog: `CHANGELOG.md`
  - Tags: `vX.Y.Z`
  - Automation: `.github/workflows/release.yml` (publishes crate, binaries, release notes)
- Schema artifact release stream:
  - Version source: `SCHEMA_VERSION`
  - Schema file: `schemas/envgen.schema.vA.B.C.json`
  - Changelog: `SCHEMA_CHANGELOG.md`
  - Tags: `schema-vA.B.C`
  - Automation: no publish workflow is triggered by schema tags

## One-time setup (crates.io trusted publishing)

Configure the crate in crates.io to trust this repository and workflow:

1. Open crate settings on crates.io for `envgen`.
2. Add a Trusted Publisher for GitHub Actions.
3. Set owner/repo to `smorinlabs/envgen`.
4. Set workflow file to `.github/workflows/release.yml`.
5. Set environment to `crates-io`.

This mapping must match the publish job in `.github/workflows/release.yml` exactly.

## Toolchain parity policy

- Rust is patch-pinned at `1.88.0` via `rust-toolchain.toml`.
- `Cargo.toml` `rust-version` is also `1.88.0`.
- CI/release workflows set Rust `1.88.0` explicitly to keep local and automation behavior aligned.

## Command reference

- `make version-status`
- `make check-core`
- `make check-msrv`
- `make check-security`
- `make check-release`
- `make sync-lockfile`
- `make check-lockfile`
- `make precommit-fast`
- `make prepush-full`
- `make bump-crate LEVEL=patch|minor|major`
- `make bump-crate VERSION=X.Y.Z`
- `make bump-crate-patch|bump-crate-minor|bump-crate-major`
- `make bump-schema LEVEL=patch|minor|major`
- `make bump-schema VERSION=A.B.C`
- `make bump-schema-patch|bump-schema-minor|bump-schema-major`
- `make bump-dry-run MODE=crate|schema LEVEL=...`
- `make bump-dry-run MODE=crate|schema VERSION=...`
- `make tag-crate`
- `make push-tag-crate`
- `make tag-schema`
- `make push-tag-schema`

## Guided hints

Release-flow commands print a `Hint:` + `Next:` block to guide the next command in the sequence.

- Default behavior:
  - Hints are shown for local interactive runs (TTY).
  - Hints are suppressed in CI/non-interactive output.
- Override behavior:
  - `ENVGEN_HINTS=1` forces hints on.
  - `ENVGEN_HINTS=0` forces hints off.

Commands with guided next-step output include:

- `make bump-crate*`
- `make check-release`
- `make tag-crate`
- `make push-tag-crate`
- `make bump-schema*`
- `make check-schema`
- `make tag-schema`
- `make push-tag-schema`

Example (crate flow):

```text
$ make bump-crate-patch
...
Hint: Crate release prep updated to vX.Y.Z.
Next:
  $ make check-release
```

```text
$ make check-release
...
✓ Release readiness checks passed for crate vX.Y.Z
Hint: Release readiness checks passed for crate vX.Y.Z.
Next:
  $ git add Cargo.toml Cargo.lock CHANGELOG.md
  $ git commit -m "chore(release): bump crate to vX.Y.Z"
  $ git push origin main
  $ make tag-crate
```

Example (schema flow):

```text
$ make bump-schema-patch
...
Hint: Schema release prep updated to vA.B.C.
Next:
  $ make check-schema
```

```text
$ make check-schema
...
Hint: Schema checks passed for artifact vA.B.C.
Next:
  $ git add SCHEMA_VERSION SCHEMA_CHANGELOG.md schemas/envgen.schema.vA.B.C.json
  $ git commit -m "chore(schema): schema-vA.B.C"
  $ git push origin main
  $ make tag-schema
```

## Quality gate matrix

| Entry point | Canonical target | Purpose |
| --- | --- | --- |
| Local commit hook | `make precommit-fast` | Fast non-mutating checks before commit |
| Local pre-push hook | `make prepush-full` | Full quality/security/MSRV checks before push |
| Local/manual core parity | `make check-core` | Core checks shared by CI Linux and release Linux verification |
| CI Linux | `make check-core` | Core release-parity verification |
| CI macOS | `make check-rust` | Rust-only portability checks |
| CI MSRV | `make check-msrv` | Declared MSRV correctness |
| CI security | `make check-security` | Audit/dependency/spelling checks |
| Release verification (Linux) | `make check-core` | Same core gate as CI Linux |
| Release verification (macOS) | `make check-rust` | Rust-only portability checks |
| Local release readiness | `make check-release` | `check-core` + `cargo publish --dry-run --locked --allow-dirty` |

## Local contributor workflow

1. Install git hooks once:
   - `make pre-commit-setup`
2. Run fast checks when iterating:
   - `make precommit-fast`
3. Run full checks before pushing:
   - `make prepush-full`
4. Before tagging a crate release:
   - `make check-release`

Tag commands are file-first by default:

- `make tag-crate` / `make push-tag-crate` read version from `Cargo.toml`.
  - `VERSION=X.Y.Z` is the only override.
- `make tag-schema` / `make push-tag-schema` read version from `SCHEMA_VERSION`.
  - `SCHEMA_VERSION=A.B.C` is the only override.

Overrides must match the corresponding file value; mismatches fail fast.

## How Bumping Works

| Command | Updates | Does not update | Tag behavior |
| --- | --- | --- | --- |
| `make bump-crate ...` | `Cargo.toml`, `CHANGELOG.md` | `SCHEMA_VERSION`, `schemas/envgen.schema.v*.json`, `SCHEMA_CHANGELOG.md` | Does not create or push tags |
| `make bump-schema ...` | `SCHEMA_VERSION`, `schemas/envgen.schema.v*.json`, `SCHEMA_CHANGELOG.md` | `Cargo.toml`, `CHANGELOG.md` | Does not create or push tags |
| `make tag-crate` / `make push-tag-crate` | Local crate tag and optional remote push | Schema version files/changelog | Uses `Cargo.toml` version by default; `VERSION=...` override must match file value; tag is `vX.Y.Z` |
| `make tag-schema` / `make push-tag-schema` | Local schema tag and optional remote push | Crate version files/changelog | Uses `SCHEMA_VERSION` file value by default; `SCHEMA_VERSION=...` override must match file value; tag is `schema-vA.B.C` |

## Tagging behavior

- `make bump-*` commands only edit files and never create/push tags.
- `make tag-*` commands create local annotated tags only.
- `make push-tag-*` commands only push existing local tags.

This is intentional to prevent accidental release workflow triggers.

## Dry-run first

Run a dry-run before applying changes:

```bash
make bump-dry-run MODE=crate LEVEL=patch
make bump-dry-run MODE=schema LEVEL=patch
```

You can also provide explicit versions:

```bash
make bump-dry-run MODE=crate VERSION=X.Y.Z
make bump-dry-run MODE=schema VERSION=A.B.C
```

## Crate bump flow (example)

1. Make sure `main` is green (CI passing).
2. Preview the crate bump:
   - `make bump-dry-run MODE=crate LEVEL=patch`
3. Apply the crate bump:
   - `make bump-crate LEVEL=patch` (or `VERSION=X.Y.Z`)
   - This updates `Cargo.toml`, `CHANGELOG.md`, and synchronizes `Cargo.lock`.
4. Validate:
   - `make check-release`
5. Commit release prep (recommended):
   - `chore(release): vX.Y.Z`
6. Create local tag:
   - `make tag-crate`
7. Push tag:
   - `make push-tag-crate`
8. Watch GitHub Actions `Release` workflow.

The release workflow at `.github/workflows/release.yml` runs when:

- You push a tag matching `v*.*.*`, or
- You manually run the workflow (`workflow_dispatch`) and provide an existing tag.

## Schema bump flow (example)

1. Update schema content as needed in `schemas/envgen.schema.v<current>.json`.
2. Preview the schema artifact bump:
   - `make bump-dry-run MODE=schema LEVEL=patch`
3. Apply schema artifact bump:
   - `make bump-schema LEVEL=patch` (or `VERSION=A.B.C`)
4. Validate:
   - `make check-schema`
5. Commit schema release prep (recommended):
   - `chore(schema): schema-vA.B.C`
6. Create local schema tag:
   - `make tag-schema`
7. Push schema tag:
   - `make push-tag-schema`

Pushing `schema-v*.*.*` tags does not trigger crates.io publish.

## Failure modes

- Missing release section for tagging:
  - `make tag-crate` fails unless `CHANGELOG.md` contains `## [X.Y.Z] - YYYY-MM-DD`.
  - `make tag-schema` fails unless `SCHEMA_CHANGELOG.md` contains `## [A.B.C] - YYYY-MM-DD`.
- Override mismatch:
  - `VERSION=... make tag-crate` fails if override does not match `Cargo.toml`.
  - `SCHEMA_VERSION=... make tag-schema` fails if override does not match `SCHEMA_VERSION`.
- Missing local tag on push:
  - `make push-tag-crate` fails if `vX.Y.Z` has not been created locally.
  - `make push-tag-schema` fails if `schema-vA.B.C` has not been created locally.
- Empty `Unreleased` section during bump:
  - Crate override: `make bump-crate-patch ALLOW_EMPTY_CHANGELOG=1`
  - Schema override: `make bump-schema-patch ALLOW_EMPTY_SCHEMA_CHANGELOG=1`
- Lockfile mismatch during locked checks:
  - Symptom: `Cargo.lock needs to be updated but --locked was passed`
  - Fix: `make sync-lockfile`
- Partial update on bump failure:
  - A failed bump can leave version/changelog edits before later steps complete.
  - Recovery: run `make sync-lockfile` and retry the bump after fixing the reported error.

## Emergency fallback (temporary)

For migration safety, token-based publish remains available through
`.github/workflows/publish-fallback.yml` (manual trigger only).

- This workflow requires the `CARGO_REGISTRY_TOKEN` secret in the `crates-io` environment.
- Keep fallback enabled for exactly 2 successful releases after Trusted Publishing goes live.
- After 2 successful releases:
  1. Remove `.github/workflows/publish-fallback.yml`.
  2. Remove `CARGO_REGISTRY_TOKEN` from GitHub secrets.

## Notes

- This setup assumes you use "Squash and merge" and "Default to PR title for squash merge commits" in GitHub settings so your main-branch commits stay Conventional.
- homebrew-core bump PRs require the `HOMEBREW_GITHUB_API_TOKEN` secret (a GitHub token able to open PRs against `Homebrew/homebrew-core`).
- The homebrew-core automation will no-op until the `envgen` formula exists in `Homebrew/homebrew-core`.

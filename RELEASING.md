# Releasing `envgen`

This repo uses:
- Conventional Commits (enforced on PR titles)
- `git-cliff` to generate GitHub Release notes
- Git tags in the form `vX.Y.Z` (e.g. `v0.2.0`)

## Release workflow trigger

The GitHub Actions workflow at `.github/workflows/release.yml` runs when:
- You push a tag matching `v*.*.*`, or
- You manually run the workflow (workflow_dispatch) and provide an existing tag.

## Release checklist

1. Make sure `main` is green (CI passing).
2. Decide the release version `X.Y.Z`.
3. Update versions:
   - `Cargo.toml` (`[package].version = "X.Y.Z"`)
   - If the embedded JSON Schema changed, consider versioning the schema file name too:
     - `schemas/envgen.schema.vX.Y.Z.json`
     - Update `SCHEMA_FILE` in `Makefile` accordingly.
4. Update `CHANGELOG.md`:
   - Move entries from `[Unreleased]` into a new `## [X.Y.Z] - YYYY-MM-DD` section.
5. Commit the release prep (recommended commit message):
   - `chore(release): vX.Y.Z`
6. Tag and push:
   - `git tag vX.Y.Z`
   - `git push origin vX.Y.Z`
7. Watch the GitHub Actions “Release” workflow:
   - Creates/updates the GitHub Release body using `git-cliff`
   - Builds and uploads binaries for Linux/macOS/Windows as release assets

## Notes

- This setup assumes you use “Squash and merge” and “Default to PR title for squash merge commits” in GitHub settings so your main-branch commits stay Conventional.
- `cargo publish` is intentionally not automated; run it manually if/when you publish to crates.io.


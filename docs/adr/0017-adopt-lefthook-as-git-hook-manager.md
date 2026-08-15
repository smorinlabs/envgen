# ADR 0017: Adopt lefthook as the git hook manager

## Status

Accepted — 2026-08-15

Supersedes the hook-manager portion of ADR-0012. The rest of ADR-0012 — the
Makefile as the single source of truth for quality gates, and every automation
layer calling those targets — is unchanged and is the reason this swap is small.

## Context

ADR-0012 chose `pre-commit` and reduced it to a wrapper: `.pre-commit-config.yaml`
declared two `language: system` hooks with `pass_filenames: false`, running
`make precommit-fast` at the pre-commit stage and `make prepush-full` at
pre-push. The hook manager holds no check logic — it only decides when to invoke
a make target.

Every other active Smorin Labs repository uses lefthook. A survey of the local
clones found 16 repositories with a `lefthook.yml` — including `toggle` (the
other Rust project), `agent-deck`, `worktreeflow`, `template-press`,
`skillsmith`, and `py-launch-blueprint` — and envgen as the only remaining
`pre-commit` holdout. That split imposes a real cost: contributors moving
between repositories need two hook managers installed, learn two bypass
mechanisms (`SKIP=` versus `LEFTHOOK=0`), and two sets of `make`/`just` target
names.

Two further facts made the timing favorable:

- No hooks were actually wired in the maintainer's checkout (`.git/hooks`
  contained only samples) and `pre-commit` was not installed on the machine, so
  the gate was not running locally regardless.
- `pre-commit` is a Python tool requiring its own install path (`uv tool`,
  `pipx`, `brew`, or `pip` — the old `install-pre-commit` target tried all four).
  lefthook is a single Go binary already present via Homebrew.

## Decision

Adopt lefthook as the git hook manager, preserving the ADR-0012 wrapper design
exactly.

- Add `lefthook.yml` declaring two commands, one per stage, calling the same
  make targets as before:
  - `pre-commit` → `make precommit-fast`
  - `pre-push` → `make prepush-full`
- Set `min_version: 2.0.0`, consistent with the version-pinning convention
  already applied to `cargo-audit@0.22.1`, `cargo-machete@0.9.2`,
  `cargo-msrv@0.19.3`, `typos-cli@1.32.0`, and `yamlfmt@v0.15.0`.
- Replace `make install-pre-commit` with `make install-lefthook` (Homebrew
  first, then the official install script — the pattern used by `worktreeflow`).
- Replace the three `pre-commit-*` targets with two `lefthook-*` targets:
  - `make pre-commit-setup` → `make lefthook-install`
  - `make pre-commit-staged` and `make pre-commit-all` → `make lefthook-run`
- Delete `.pre-commit-config.yaml`.
- Update the `RELEASING.md` contributor workflow and command reference.

Check definitions stay in the Makefile. New checks are added to
`precommit-fast` or `prepush-full`, never to `lefthook.yml`.

## Consequences

- One hook manager across the organization; contributors install lefthook once.
- The bypass idiom changes from `SKIP=<hook-id> git commit` to
  `LEFTHOOK=0 git commit`.
- CI and release workflows are untouched — they call `make check-core`,
  `check-rust`, `check-msrv`, and `check-security` directly and never invoked
  the hook manager.
- **The commit and push paths behave identically.** Measured on throwaway clones
  of both revisions: a clean commit passes under each manager; a `cargo fmt`
  violation blocks the commit and leaves `HEAD` unmoved under each; and a real
  `git push` fires the full `prepush-full` gate under each, failing at the same
  check and refusing the push.
- **Both managers skip a stage when its file list is empty.** lefthook reports
  `precommit-fast (skip) no matching staged files` and exits 0 when nothing is
  staged, the same way `pre-commit` reports `(no files to check) Skipped`. This
  is why `make lefthook-run` passes `--all-files`: without it the target is a
  silent no-op on a clean tree rather than a manual gate. `make precommit-fast`
  remains the direct way to run the checks with no hook manager involved.
- **Manual pre-push invocation is the one stage that differs.**
  `pre-commit run --hook-stage pre-push` skipped and exited 0 with no file
  range supplied; `lefthook run pre-push` runs the gate. Real pushes were
  unaffected — both fired.
- **Unstaged changes are no longer hidden during the pre-commit gate.**
  `pre-commit` stashes the unstaged working tree before running hooks; lefthook
  does not. This was measured, not assumed: with an unstaged `cargo fmt`
  violation in `src/main.rs` and only a clean file staged, `pre-commit` reported
  `Passed` while lefthook failed the commit. Because `precommit-fast` checks the
  whole tree rather than a staged file list, lefthook reports problems in dirty
  working-tree files that `pre-commit` hid. This makes the local gate agree with
  what CI sees after the branch is pushed, at the cost of occasionally failing a
  commit over a file that is not part of it.
- `lefthook install` leaves no untracked files behind, so `.gitignore` is
  unchanged.
- `make pre-commit-staged` and `make pre-commit-all` are removed rather than
  renamed. With `pass_filenames: false` they were already indistinguishable —
  both ran `make precommit-fast` over the whole tree — so the staged-versus-all
  distinction never existed in this repository. `RELEASING.md` referenced
  neither.
- The `UV_CACHE_DIR` and `UV_TOOL_DIR` variables are no longer used for hook
  tooling. They remain in use by `yamllint` and `check-jsonschema` via `uvx`.

## Alternatives considered

- **Per-tool lefthook config**, listing `cargo fmt`, `clippy`, `actionlint`, and
  `yamllint` individually with `glob:` and `{staged_files}` scoping, as
  `toggle` and `py-launch-blueprint` do. Faster commits, since only staged files
  are checked. Rejected: it duplicates the gate definition that ADR-0012
  deliberately centralized in the Makefile, and it would make the local hook
  check a different set of files than CI. Scoping can be revisited later by
  changing the Makefile targets, which keeps CI in lockstep automatically.
- **Stay on `pre-commit`.** No migration cost, but leaves envgen as the only
  repository requiring a second hook manager, and keeps a four-branch Python
  installer for a tool used solely to shell out to `make`.
- **Drop hook management entirely** and rely on CI. Rejected: `prepush-full`
  catches MSRV and security regressions before the push round-trip, which is
  the gate's main value.

## References/links

- `/Users/stevemorin/c/envgen/lefthook.yml`
- `/Users/stevemorin/c/envgen/Makefile`
- `/Users/stevemorin/c/envgen/RELEASING.md`
- `docs/adr/0012-unify-ci-release-and-precommit-conventions.md`
- lefthook configuration reference: https://lefthook.dev/configuration/

# envgen push — Design Spec

**Date:** 2026-04-27
**Status:** Approved (awaiting implementation plan)

## Goal

Add a `push` subcommand to envgen — the inverse of `pull`. Where `pull` resolves a variable's value from its source for a chosen environment, `push` writes a new value back to that source for a chosen environment. The schema's existing per-environment configuration (project IDs, app slugs) is reused so the user never has to remember which gcloud project corresponds to which env.

## Motivation

Today, rotating or setting a secret in Secret Manager requires the user to remember the right `--project=` and `--secret=` flags by hand. envgen already encodes that mapping in `environments.<env>.*` and the source's `command:` template. `push` lets the user say `--env stg` and have envgen fill in the rest, identically to how `pull` already does.

## Non-goals (v1)

- Bulk push of multiple keys in one invocation.
- Read-back verification (running `command:` after `push_command:` to confirm round-trip).
- Push semantics for `static` or `manual` sources.
- Audit logging beyond a success/failure line.
- Prompt timeout.

## Schema additions

Single additive field on `Source`:

```rust
pub struct Source {
    pub command: String,
    pub push_command: Option<String>, // NEW
    pub label: Option<String>,
    pub url: Option<String>,
    pub description: Option<String>,
}
```

YAML example:

```yaml
sources:
  gcloud:
    command:      "gcloud secrets versions access latest --secret={key} --project={app_slug}"
    push_command: "gcloud secrets versions add {key} --data-file=- --project={app_slug}"
```

Rules:
- `push_command` is optional. Sources without it cannot be pushed to.
- The same placeholder set as `command:` applies: `{key}`, `{environment}`, and per-env config keys from `environments.<env>.*`.
- **There is no `{value}` placeholder.** The new value is delivered via the child process's stdin — never substituted into the command string. This avoids shell-history exposure, quoting hazards, and the temptation to write `echo {value} | foo`. The convention (matching gcloud, `op`, and most secret CLIs) is `--data-file=-` or an equivalent stdin sink.
- Adding `push_command` is backwards-compatible (additive `serde(default)`); no schema version bump.

Validation:
- If `push_command` is present, validate its template against the same context as `command:`.
- The JSON Schema (`schemas/envgen.schema.v0.1.0.json`) is updated additively to declare the field.

## CLI surface

```
envgen push -c <YAML> --env <ENV> <VAR_NAME>
            [--from-file <PATH>]
            [--yes]
            [--show-secret]
            [--dry-run]
            [--source-timeout 30]
            [--allow-empty]
```

Exactly one variable per invocation.

### Value input modes (auto-detected, mutually exclusive)

1. `--from-file <PATH>` provided → read file contents, treat as value.
2. stdin is **not** a TTY → read stdin to EOF, treat as value.
3. stdin **is** a TTY → interactive hidden prompt: `Enter value for <VAR_NAME>:`.

In modes 1 and 2, exactly one trailing newline (`\n` or `\r\n`) is stripped from the value before piping. This matches shell `$(cmd)` semantics and the common case `echo "secret" > file`. Users who genuinely need a trailing newline in the stored secret are out of luck in v1; this is rare enough to not be worth a flag. Mode 3 (interactive prompt) reads a single line and uses it verbatim.

When `--from-file` is provided, stdin is ignored. There is no separate ambiguity error: file beats pipe. (We can't reliably distinguish "user piped data" from "test harness closed stdin" via `IsTerminal`, so the simpler rule wins.)

### Empty-value guard

Empty values are refused by default with exit 1: `Refusing to push empty value for '<VAR>'. Pass --allow-empty to override.` `--allow-empty` opts in.

### Confirmation prompt (non-local environments only)

Triggered when the target env name is not `local`, the user did not pass `--yes`, and the run is not `--dry-run`. Output:

```
About to push to non-local environment:
  variable:    STRIPE_SECRET_KEY
  environment: stg
  source:      gcloud (Google Cloud Secret Manager)   # "(label)" suffix shown only when source.label is set
  command:     gcloud secrets versions add STRIPE_SECRET_KEY --data-file=- --project=acme-stg
  value:       ******** (use --show-secret to reveal)
Continue? [y/N]:
```

The check is `env_name != "local"`. It is a speed bump, not a safety system; users who rename their prod env to `local` are responsible for that. `--yes` always overrides.

### Secret display

Mirrors `pull --show-secrets`. Default: secret value is masked everywhere (confirmation prompt, dry-run preview, error output). With `--show-secret`, the value is printed unmasked. The resolved command line itself is always shown — only the value piped over stdin is masked.

### Exit codes

| Code | Meaning |
|---|---|
| 0 | Push succeeded. |
| 1 | Schema, CLI, or validation error; missing `push_command`; empty value without `--allow-empty`; user declined confirmation. |
| 2 | The push command itself exited non-zero or timed out. |

Splitting 1 vs 2 lets CI distinguish "you gave envgen bad inputs" from "the remote system rejected the write."

## Resolution flow

Mirrors the resolver chain in `pull` (see `src/schema/types.rs:127-131`):

1. Load and validate the schema. Confirm `<ENV>` exists in `metadata.destination` and `environments`.
2. Look up `<VAR_NAME>` in `variables`. Error if missing.
3. Check `variable.applies_to(env)`. Error if the variable's `environments:` list excludes the target env.
4. Call `effective_source_for_env(env)` to pick the active source — resolver-aware, identical to `pull`.
5. Branch on the source:
   - `static` → emit the static-specific error.
   - `manual` → emit the manual-specific error.
   - named source → look up the `Source`. If `push_command` is `None`, emit the schema-fix error with a copy-pasteable YAML snippet.
6. Read the value (one of the three input modes). Apply the empty-value guard.
7. Build the resolved push command via `template::expand_template` with `{key} = effective_key_for_env(...)`, plus `{environment}` and the env config map.
8. If non-local and not `--yes` and not `--dry-run`, prompt for confirmation.
9. Execute via a new `execute_command_with_stdin(cmd, stdin, timeout)` helper. Capture stdout/stderr.
10. On success, print one line: `✓ Pushed <VAR_NAME> to <ENV> via <source_name>`. Exit 0.

## Error catalog

| Condition | Exit | Message shape |
|---|---|---|
| Variable not in schema | 1 | `Variable '<VAR>' not found in schema` |
| Variable not applicable to env | 1 | `Variable '<VAR>' is not applicable to env '<ENV>' (allowed: [<envs>])` |
| Source is `static` | 1 | `Cannot push '<VAR>' for env=<ENV>: source is 'static'. Static values are defined inline — edit the variable's values: block.` |
| Source is `manual` | 1 | `Cannot push '<VAR>' for env=<ENV>: source is 'manual'. Manual sources have no remote — store the value in your password manager.` |
| Source has no `push_command` | 1 | `Cannot push '<VAR>' for env=<ENV>. Source '<SOURCE>' has no push_command defined.\n\nAdd to your schema:\n  sources:\n    <SOURCE>:\n      push_command: "<example, e.g. gcloud secrets versions add {key} --data-file=- --project={app_slug}>"` |
| Empty value without `--allow-empty` | 1 | `Refusing to push empty value for '<VAR>'. Pass --allow-empty to override.` |
| User declined prompt | 1 | `Push cancelled.` |
| Push command exited non-zero | 2 | `Push command failed (exit <N>): <stderr>\nResolved command: <expanded_cmd>` (value redacted unless `--show-secret`) |
| Push command timed out | 2 | `Push command timed out after <N> seconds.` |

## Code layout

### New files
- `src/commands/push.rs` — `PushOptions` struct, `run_push(opts) -> Result<i32>`. Mirror of `src/commands/pull.rs`. Owns: arg-parsing structs, value-input-mode detection, confirmation prompt, output formatting.

### Modified files
- `src/main.rs` — add `Push { ... }` variant to the `Commands` enum and dispatch arm.
- `src/commands/mod.rs` — `pub mod push;`.
- `src/schema/types.rs` — add `push_command: Option<String>` to `Source` (with `#[serde(default)]`).
- `src/schema/validation.rs` — when a source defines `push_command`, validate the template against the same context as `command:`.
- `src/resolver/command_source.rs` — generalize `execute_command` to `execute_command_with_stdin(cmd, stdin: Option<&str>, timeout_secs: u64)`. Keep an `execute_command(cmd, timeout)` wrapper that delegates with `None`, so `pull.rs` does not change.
- `schemas/envgen.schema.v0.1.0.json` — additive: declare optional `push_command` on the source object.
- `schemas/envgen.sample.yaml` — add `push_command:` on the `gcloud` source as a documented example.
- `README.md` and relevant `docs/` — usage section.

### Module boundaries
- `commands::push` is the only module that knows about TTY detection, prompts, and CLI shape.
- `resolver::command_source` stays push/pull-agnostic — it executes a command, optionally with stdin.
- `schema::*` validates structure; it does not know push exists beyond "validate this template if present."

## Testing

### Unit tests (in-module `#[cfg(test)]`)
- `command_source::tests::execute_command_with_stdin_pipes_value` — fake command (`cat > $TMPFILE`), assert file contents match the piped value.
- `command_source::tests::execute_command_with_stdin_none_acts_like_old` — regression covering `pull`'s call path.
- `push::tests::missing_push_command_error_includes_yaml_snippet` — assert the error message contains a copy-pasteable YAML block.
- `push::tests::static_source_error_message`.
- `push::tests::manual_source_error_message`.
- `push::tests::resolver_chain_picks_correct_source_for_env` — variable with `resolvers:`; assert stg picks `gcloud`, local picks `manual`, and the manual path emits the manual-specific error.
- `push::tests::ambiguous_value_source_errors` — `--from-file` + non-TTY stdin returns the ambiguity error.
- `push::tests::empty_value_refused_without_flag`.

### Integration tests (`tests/`)
- `push_writes_to_fake_secret_store` — schema with `push_command: "tee {key}.txt > /dev/null"` in a tmpdir; run push with `--from-file`; assert the file contains the value.
- `push_dry_run_does_not_execute` — same, but with `--dry-run`; assert no file written and the resolved command is printed.
- `push_non_local_requires_confirmation` — drive stdin to send `n`; assert exit 1 and `Push cancelled`.
- `push_yes_skips_confirmation` — same scenario with `--yes`; assert success.
- `push_command_failure_exit_code_2` — `push_command: "exit 1"`; assert exit code 2 and stderr appears in the error message.
- `push_show_secret_unmasks_in_dry_run`.

## Open questions

None. All decisions captured above were confirmed during brainstorming.

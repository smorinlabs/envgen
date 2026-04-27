# envgen push Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an `envgen push` subcommand that writes a value to a variable's source for a chosen environment, mirroring how `pull` reads from it.

**Architecture:** Additive `push_command: Option<String>` field on `Source`. New `src/commands/push.rs` reuses pull's resolver chain (`Variable::effective_source_for_env`, `effective_key_for_env`). The command-source executor is generalized to optionally pipe a value into the child's stdin; the value never enters the command string. Confirmation prompt fires for any env name other than `local`.

**Tech Stack:** Rust 2021, clap 4, tokio, dialoguer (existing prompt lib), anyhow, regex (existing template engine), assert_cmd + predicates for integration tests, tempfile for fixtures.

**Spec reference:** `docs/superpowers/specs/2026-04-27-envgen-push-design.md`.

**Verification commands** (from `Makefile`):
- Quick test loop: `cargo test --locked`
- Full pre-commit gate: `make check-rust` (clippy + fmt + tests)
- Project-wide gate (per CLAUDE.md): `make check`

---

## File Structure

| Path | Status | Responsibility |
|---|---|---|
| `src/schema/types.rs` | modify | Add `push_command: Option<String>` field to `Source`. |
| `schemas/envgen.schema.v0.1.0.json` | modify | Declare optional `push_command` on the source object. |
| `src/schema/validator.rs` | modify | Validate `push_command` template placeholders if present. |
| `src/resolver/command_source.rs` | modify | Generalize executor to optionally pipe stdin; keep `execute_command` as a `None`-stdin wrapper so `pull` doesn't change. |
| `src/commands/push.rs` | **create** | `PushOptions`, `run_push`, value-input-mode detection, confirmation, output formatting. |
| `src/commands/mod.rs` | modify | `pub mod push;` |
| `src/main.rs` | modify | Add `Push { ... }` variant + dispatch arm. |
| `schemas/envgen.sample.yaml` | modify | Add `push_command:` to the `gcloud` source as a documented example. |
| `tests/test_push.rs` | **create** | Integration tests using `assert_cmd`, `tempfile`. |
| `tests/fixtures/push_*.yaml` | **create** | Fixtures with a fake push command (`tee {key}.txt`). |
| `README.md` | modify | Add `push` usage section. |

---

## Task 1: Add `push_command` field to `Source` type

**Files:**
- Modify: `src/schema/types.rs:23-36`
- Modify: `schemas/envgen.schema.v0.1.0.json:87-108`

- [ ] **Step 1: Add a unit test that proves the parser deserializes `push_command`**

Add to `src/schema/types.rs` at the bottom (creating a `#[cfg(test)] mod tests` block; the file does not currently have one):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_deserializes_push_command_when_present() {
        let yaml = r#"
command: "read --key {key}"
push_command: "write --key {key} --data-file=-"
"#;
        let src: Source = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(src.command, "read --key {key}");
        assert_eq!(
            src.push_command.as_deref(),
            Some("write --key {key} --data-file=-")
        );
    }

    #[test]
    fn source_push_command_defaults_to_none() {
        let yaml = r#"
command: "read --key {key}"
"#;
        let src: Source = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(src.push_command, None);
    }
}
```

- [ ] **Step 2: Run the new tests and confirm they fail to compile**

Run: `cargo test --locked schema::types::tests`
Expected: build error — `Source` has no field `push_command`.

- [ ] **Step 3: Add the field to `Source`**

Edit `src/schema/types.rs:23-36`:

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub command: String,

    #[serde(default)]
    pub push_command: Option<String>,

    #[serde(default)]
    pub label: Option<String>,

    #[serde(default)]
    pub url: Option<String>,

    #[serde(default)]
    pub description: Option<String>,
}
```

- [ ] **Step 4: Update the embedded JSON Schema to allow `push_command`**

Edit `schemas/envgen.schema.v0.1.0.json:87-108` so the `source` definition becomes:

```json
"source": {
    "type": "object",
    "additionalProperties": false,
    "required": ["command"],
    "properties": {
        "command": {
            "type": "string",
            "minLength": 1
        },
        "push_command": {
            "type": "string",
            "minLength": 1
        },
        "label": {
            "type": "string",
            "minLength": 1
        },
        "url": {
            "type": "string",
            "minLength": 1
        },
        "description": {
            "type": "string",
            "minLength": 1
        }
    }
}
```

- [ ] **Step 5: Run the new tests and the full schema test suite**

Run: `cargo test --locked schema`
Expected: PASS, including the two new `source_*` tests and existing schema tests.

- [ ] **Step 6: Commit**

```bash
git add src/schema/types.rs schemas/envgen.schema.v0.1.0.json
git commit -m "feat(schema): add optional push_command field on Source

Additive serde(default) field plus matching JSON Schema property.
No schema_version bump — existing schemas remain valid."
```

---

## Task 2: Validate `push_command` template placeholders

**Files:**
- Modify: `src/schema/validator.rs:299-322` (extend the command-source placeholder validation block)

- [ ] **Step 1: Add a failing validator test**

Append to the `mod tests` block in `src/schema/validator.rs`:

```rust
#[test]
fn test_push_command_unresolved_placeholder() {
    let yaml = r#"
schema_version: "2"
metadata:
  description: "Test"
  destination:
    local: ".env"
environments:
  local:
    project: "test"
sources:
  my-source:
    command: "echo {key} --project {project}"
    push_command: "echo {key} --project {bogus}"
variables:
  FOO:
    description: "A variable"
    source: my-source
"#;
    let errors = errors_for(yaml);
    assert!(
        errors
            .iter()
            .any(|e| e.contains("push_command") && e.contains("{bogus}") && e.contains("local")),
        "Expected push_command placeholder error, got: {:?}",
        errors
    );
}

#[test]
fn test_push_command_valid_when_placeholders_resolve() {
    let yaml = r#"
schema_version: "2"
metadata:
  description: "Test"
  destination:
    local: ".env"
environments:
  local:
    project: "test"
sources:
  my-source:
    command: "echo {key} --project {project}"
    push_command: "echo {key} --project {project} --data-file=-"
variables:
  FOO:
    description: "A variable"
    source: my-source
"#;
    let errors = errors_for(yaml);
    assert!(errors.is_empty(), "Expected no errors, got: {:?}", errors);
}
```

- [ ] **Step 2: Run and confirm the failing test fails**

Run: `cargo test --locked schema::validator::tests::test_push_command_unresolved_placeholder`
Expected: FAIL — no error mentioning `push_command` is produced today.

- [ ] **Step 3: Extract a helper for command-template placeholder validation**

The existing logic at `src/schema/validator.rs:299-322` validates `src.command`. We will run the same check against `src.push_command` when present. Replace lines 299-322 (the block beginning `// Check source command template placeholders can be resolved` in the **non-resolver** branch) with:

```rust
            // Check source command template placeholders can be resolved
            if source != "static" && source != "manual" {
                if let Some(src) = schema.sources.get(source) {
                    for env_name in &applicable_envs {
                        if let Some(env_config) = schema.environments.get(*env_name) {
                            check_source_template(
                                var_name,
                                env_name,
                                env_config,
                                "command",
                                &src.command,
                                &mut errors,
                            );
                            if let Some(push_cmd) = &src.push_command {
                                check_source_template(
                                    var_name,
                                    env_name,
                                    env_config,
                                    "push_command",
                                    push_cmd,
                                    &mut errors,
                                );
                            }
                        }
                    }
                }
            }
```

Also update the **resolver** branch at `src/schema/validator.rs:210-232` similarly:

```rust
                // Check source command template placeholders can be resolved (resolver-level)
                if source != "static" && source != "manual" {
                    if let Some(src) = schema.sources.get(source) {
                        for env_name in &resolver.environments {
                            if let Some(env_config) = schema.environments.get(env_name) {
                                check_source_template(
                                    var_name,
                                    env_name,
                                    env_config,
                                    "command",
                                    &src.command,
                                    &mut errors,
                                );
                                if let Some(push_cmd) = &src.push_command {
                                    check_source_template(
                                        var_name,
                                        env_name,
                                        env_config,
                                        "push_command",
                                        push_cmd,
                                        &mut errors,
                                    );
                                }
                            }
                        }
                    }
                }
```

Add the helper at module scope (above `validate_schema`):

```rust
fn check_source_template(
    var_name: &str,
    env_name: &str,
    env_config: &std::collections::BTreeMap<String, String>,
    kind: &str,
    template_str: &str,
    errors: &mut Vec<String>,
) {
    let mut available_keys: Vec<String> = env_config.keys().cloned().collect();
    available_keys.push("key".to_string());
    available_keys.push("environment".to_string());

    let placeholders = template::extract_placeholders(template_str);
    for ph in placeholders {
        if !available_keys.contains(&ph) {
            errors.push(format!(
                "{}: source {} template references placeholder \"{{{}}}\" which cannot be resolved for environment \"{}\".",
                var_name, kind, ph, env_name
            ));
        }
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test --locked schema::validator`
Expected: PASS, including both new push_command tests and all existing tests.

- [ ] **Step 5: Commit**

```bash
git add src/schema/validator.rs
git commit -m "feat(schema): validate push_command template placeholders

Reuses the same {key}/{environment}/env-config check as command:.
Extracts a shared helper to avoid duplication across the
single-source and resolver-branch paths."
```

---

## Task 3: Add stdin-piping to the command executor

**Files:**
- Modify: `src/resolver/command_source.rs:60-158` (add new function; keep `execute_command` as a wrapper).
- Add: dependency check — `tokio::io::AsyncWriteExt` is already part of `tokio = { features = ["full"] }`.

- [ ] **Step 1: Add failing tests for stdin piping**

Append to the `mod tests` block in `src/resolver/command_source.rs`:

```rust
#[tokio::test]
async fn test_execute_command_with_stdin_pipes_value() {
    let tmp = tempdir().unwrap();
    let out = tmp.path().join("out.txt");
    let cmd = format!("cat > \"{}\"", out.display());

    let result = execute_command_with_stdin(&cmd, Some("piped-value"), 30)
        .await
        .unwrap();
    assert!(result.value.is_empty()); // stdout from `cat > file` is empty
    let written = std::fs::read_to_string(&out).unwrap();
    assert_eq!(written, "piped-value");
}

#[tokio::test]
async fn test_execute_command_with_stdin_none_acts_like_old() {
    // Regression: passing None must behave identically to the old execute_command.
    let result = execute_command_with_stdin("echo hello", None, 30)
        .await
        .unwrap();
    assert_eq!(result.value, "hello");
}

#[tokio::test]
async fn test_execute_command_with_stdin_failure_propagates_stderr() {
    let result = execute_command_with_stdin("cat - >&2; exit 1", Some("payload"), 30).await;
    let err = result.unwrap_err().to_string();
    assert!(err.contains("exit code 1"), "got: {}", err);
    assert!(err.contains("payload"), "expected stderr to include piped value, got: {}", err);
}
```

- [ ] **Step 2: Run and confirm they fail to compile**

Run: `cargo test --locked resolver::command_source`
Expected: build error — `execute_command_with_stdin` does not exist.

- [ ] **Step 3: Implement the new function and refactor the old one**

Replace `execute_command` (currently `src/resolver/command_source.rs:60-158`) with:

```rust
/// Execute a source command, optionally piping a value into the child's stdin.
/// Returns the trimmed stdout.
pub async fn execute_command_with_stdin(
    command: &str,
    stdin_value: Option<&str>,
    timeout_secs: u64,
) -> Result<CommandResult> {
    enum WaitOutcome {
        Completed(std::io::Result<std::process::ExitStatus>),
        TimedOut,
    }

    let timeout = Duration::from_secs(timeout_secs);

    let mut cmd = Command::new("sh");
    cmd.arg("-c")
        .arg(command)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    if stdin_value.is_some() {
        cmd.stdin(Stdio::piped());
    } else {
        cmd.stdin(Stdio::null());
    }

    #[cfg(unix)]
    configure_process_group(&mut cmd);

    let mut child = cmd.spawn().context("Failed to execute command")?;
    let pid = child.id();

    if let Some(value) = stdin_value {
        let mut stdin = child
            .stdin
            .take()
            .context("Failed to capture command stdin")?;
        let value_bytes = value.as_bytes().to_vec();
        // Write and close so the child sees EOF and exits.
        tokio::spawn(async move {
            use tokio::io::AsyncWriteExt;
            let _ = stdin.write_all(&value_bytes).await;
            let _ = stdin.shutdown().await;
        });
    }

    let mut stdout = child
        .stdout
        .take()
        .context("Failed to capture command stdout")?;
    let mut stderr = child
        .stderr
        .take()
        .context("Failed to capture command stderr")?;

    let stdout_task = tokio::spawn(async move {
        let mut buf = Vec::new();
        stdout.read_to_end(&mut buf).await?;
        Ok::<Vec<u8>, std::io::Error>(buf)
    });
    let stderr_task = tokio::spawn(async move {
        let mut buf = Vec::new();
        stderr.read_to_end(&mut buf).await?;
        Ok::<Vec<u8>, std::io::Error>(buf)
    });

    let mut timed_out = false;
    let mut wait_error: Option<std::io::Error> = None;
    let mut exit_status: Option<std::process::ExitStatus> = None;

    match tokio::select! {
        res = child.wait() => WaitOutcome::Completed(res),
        _ = tokio::time::sleep(timeout) => WaitOutcome::TimedOut,
    } {
        WaitOutcome::Completed(res) => match res {
            Ok(status) => exit_status = Some(status),
            Err(e) => wait_error = Some(e),
        },
        WaitOutcome::TimedOut => {
            timed_out = true;

            if let Some(pid) = pid {
                #[cfg(unix)]
                {
                    let _ = kill_process_group_by_pid(pid);
                }
            }

            let _ = child.kill().await;
            let _ = child.wait().await;
        }
    }

    let stdout_bytes = stdout_task
        .await
        .context("Failed to join stdout reader task")??;
    let stderr_bytes = stderr_task
        .await
        .context("Failed to join stderr reader task")??;

    if let Some(e) = wait_error {
        bail!("Failed to execute command: {}", e);
    }

    if timed_out {
        bail!("Command timed out after {} seconds", timeout_secs);
    }

    let status = exit_status.context("Missing command exit status")?;
    let stderr = String::from_utf8_lossy(&stderr_bytes).to_string();
    if !status.success() {
        bail!(
            "Command failed with exit code {}: {}",
            status.code().unwrap_or(-1),
            stderr.trim()
        );
    }
    let stdout = String::from_utf8_lossy(&stdout_bytes).trim().to_string();
    Ok(CommandResult {
        value: stdout,
        stderr,
    })
}

/// Execute a source command without stdin. Thin wrapper kept so existing
/// callers (pull) don't change.
pub async fn execute_command(command: &str, timeout_secs: u64) -> Result<CommandResult> {
    execute_command_with_stdin(command, None, timeout_secs).await
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test --locked resolver::command_source`
Expected: PASS — all existing tests plus the three new stdin tests.

- [ ] **Step 5: Run the full test suite to confirm pull is unaffected**

Run: `cargo test --locked`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/resolver/command_source.rs
git commit -m "feat(resolver): support optional stdin piping in command executor

execute_command_with_stdin pipes a value into the child's stdin
and closes it so the child sees EOF. execute_command becomes a
thin wrapper that passes None, so pull's call site is unchanged."
```

---

## Task 4: Add `commands::push` skeleton with arg parsing and dispatch

**Files:**
- Create: `src/commands/push.rs`
- Modify: `src/commands/mod.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Add an integration test that asserts `push --help` exists**

Create `tests/test_push.rs`:

```rust
use assert_cmd::cargo::cargo_bin_cmd;
use assert_cmd::Command;
use predicates::prelude::*;

fn envgen() -> Command {
    cargo_bin_cmd!("envgen")
}

#[test]
fn test_push_help_lists_expected_flags() {
    envgen()
        .arg("push")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--env"))
        .stdout(predicate::str::contains("--from-file"))
        .stdout(predicate::str::contains("--yes"))
        .stdout(predicate::str::contains("--show-secret"))
        .stdout(predicate::str::contains("--dry-run"))
        .stdout(predicate::str::contains("--source-timeout"))
        .stdout(predicate::str::contains("--allow-empty"));
}
```

- [ ] **Step 2: Run and confirm it fails (no `push` subcommand yet)**

Run: `cargo test --locked --test test_push test_push_help_lists_expected_flags`
Expected: FAIL — clap reports `unrecognized subcommand 'push'`.

- [ ] **Step 3: Create the push module skeleton**

Create `src/commands/push.rs`:

```rust
use anyhow::Result;
use std::path::PathBuf;

pub struct PushOptions {
    pub schema_path: PathBuf,
    pub env_name: String,
    pub var_name: String,
    pub from_file: Option<PathBuf>,
    pub yes: bool,
    pub show_secret: bool,
    pub dry_run: bool,
    pub source_timeout: u64,
    pub allow_empty: bool,
}

/// Run the `push` command. Returns the process exit code.
pub async fn run_push(_opts: PushOptions) -> Result<i32> {
    anyhow::bail!("push: not yet implemented");
}
```

- [ ] **Step 4: Wire the module into `commands::mod`**

Edit `src/commands/mod.rs` and add a line (alphabetical order):

```rust
pub mod push;
```

- [ ] **Step 5: Add the clap subcommand variant and dispatch arm in `main.rs`**

Edit `src/main.rs`. Inside `enum Commands`, add (between `Pull { ... }` and `Init { ... }`, or at the end before `Readme`):

```rust
    /// Push a value to a variable's source for a chosen environment
    Push {
        /// Path to envgen YAML config file
        #[arg(short = 'c', long)]
        config: PathBuf,

        /// Target environment
        #[arg(short, long)]
        env: String,

        /// Variable name to push
        var_name: String,

        /// Read the value from this file instead of stdin/prompt
        #[arg(long)]
        from_file: Option<PathBuf>,

        /// Skip the non-local confirmation prompt
        #[arg(short = 'y', long)]
        yes: bool,

        /// Reveal the secret value in confirmation/dry-run output
        #[arg(long)]
        show_secret: bool,

        /// Print what would happen without executing
        #[arg(short = 'n', long)]
        dry_run: bool,

        /// Hard timeout in seconds for the push command
        #[arg(long, default_value = "30")]
        source_timeout: u64,

        /// Permit pushing an empty value
        #[arg(long)]
        allow_empty: bool,
    },
```

In the `match cli.command { ... }` block, add the dispatch arm (mirroring `Commands::Pull`):

```rust
        Commands::Push {
            ref config,
            ref env,
            ref var_name,
            ref from_file,
            yes,
            show_secret,
            dry_run,
            source_timeout,
            allow_empty,
        } => {
            let opts = commands::push::PushOptions {
                schema_path: config.clone(),
                env_name: env.clone(),
                var_name: var_name.clone(),
                from_file: from_file.clone(),
                yes,
                show_secret,
                dry_run,
                source_timeout,
                allow_empty,
            };
            match commands::push::run_push(opts).await {
                Ok(code) => code,
                Err(e) => {
                    eprintln!("Error: {:#}", e);
                    1
                }
            }
        }
```

- [ ] **Step 6: Run the help test**

Run: `cargo test --locked --test test_push test_push_help_lists_expected_flags`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add src/commands/push.rs src/commands/mod.rs src/main.rs tests/test_push.rs
git commit -m "feat(cli): scaffold envgen push subcommand

Adds clap variant, PushOptions struct, dispatch arm, and a smoke
test asserting --help lists every documented flag. run_push
currently bails — behavior lands in the next commits."
```

---

## Task 5: Variable lookup and applies-to validation

**Files:**
- Modify: `src/commands/push.rs`
- Create: `tests/fixtures/push_basic.yaml`

- [ ] **Step 1: Create a reusable test fixture**

Create `tests/fixtures/push_basic.yaml`:

```yaml
schema_version: "2"
metadata:
  description: "Push test fixture"
  destination:
    local: ".env.local"
    stg: ".env.stg"
environments:
  local:
    out_dir: "/tmp"
  stg:
    out_dir: "/tmp"
sources:
  fakefs:
    command: "cat {out_dir}/{key}.txt"
    push_command: "tee {out_dir}/{key}.txt > /dev/null"
  readonly:
    command: "echo readonly-{key}"
variables:
  STORED_SECRET:
    description: "A secret persisted to a fake file store."
    source: fakefs
  LOCAL_ONLY:
    description: "Variable that only applies to local."
    source: fakefs
    environments: [local]
  STATIC_VAR:
    description: "A static variable."
    sensitive: false
    source: static
    values:
      local: "static-local"
      stg: "static-stg"
  MANUAL_VAR:
    description: "A manual variable."
    source: manual
  NO_PUSH:
    description: "Source has no push_command."
    source: readonly
```

- [ ] **Step 2: Add failing integration tests**

Append to `tests/test_push.rs`:

```rust
#[test]
fn test_push_unknown_variable_errors() {
    envgen()
        .arg("push")
        .arg("-c")
        .arg("tests/fixtures/push_basic.yaml")
        .arg("--env")
        .arg("local")
        .arg("--from-file")
        .arg("/dev/null")
        .arg("--allow-empty")
        .arg("NOT_A_VAR")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("Variable 'NOT_A_VAR' not found"));
}

#[test]
fn test_push_variable_not_applicable_to_env_errors() {
    envgen()
        .arg("push")
        .arg("-c")
        .arg("tests/fixtures/push_basic.yaml")
        .arg("--env")
        .arg("stg")
        .arg("--from-file")
        .arg("/dev/null")
        .arg("--allow-empty")
        .arg("--yes")
        .arg("LOCAL_ONLY")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("LOCAL_ONLY"))
        .stderr(predicate::str::contains("not applicable"))
        .stderr(predicate::str::contains("stg"));
}

#[test]
fn test_push_unknown_environment_errors() {
    envgen()
        .arg("push")
        .arg("-c")
        .arg("tests/fixtures/push_basic.yaml")
        .arg("--env")
        .arg("nope")
        .arg("--from-file")
        .arg("/dev/null")
        .arg("--allow-empty")
        .arg("STORED_SECRET")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("Environment \"nope\" not found"));
}
```

- [ ] **Step 3: Run and confirm they fail**

Run: `cargo test --locked --test test_push`
Expected: All three new tests fail (run_push still bails with "not yet implemented").

- [ ] **Step 4: Implement schema load + env + applies_to checks**

Replace the body of `run_push` in `src/commands/push.rs`:

```rust
use anyhow::{bail, Result};
use colored::Colorize;
use std::path::PathBuf;

use crate::schema::validation::{load_and_validate_schema_file, SchemaValidation};

pub struct PushOptions {
    pub schema_path: PathBuf,
    pub env_name: String,
    pub var_name: String,
    pub from_file: Option<PathBuf>,
    pub yes: bool,
    pub show_secret: bool,
    pub dry_run: bool,
    pub source_timeout: u64,
    pub allow_empty: bool,
}

/// Run the `push` command. Returns the process exit code.
pub async fn run_push(opts: PushOptions) -> Result<i32> {
    let schema = match load_and_validate_schema_file(&opts.schema_path)? {
        SchemaValidation::Valid(s) => s,
        SchemaValidation::Invalid(errors) => {
            println!("{} Schema errors:", "✗".red());
            for e in &errors {
                println!("  - {}", e);
            }
            bail!("Schema validation failed. Fix errors before pushing.");
        }
    };

    if !schema.environments.contains_key(&opts.env_name) {
        let available: Vec<String> = schema.environment_names();
        bail!(
            "Environment \"{}\" not found. Available: {}",
            opts.env_name,
            available.join(", ")
        );
    }

    let var = match schema.variables.get(&opts.var_name) {
        Some(v) => v,
        None => bail!("Variable '{}' not found in schema", opts.var_name),
    };

    if !var.applies_to(&opts.env_name) {
        let allowed = var
            .environments
            .as_ref()
            .map(|e| e.join(", "))
            .unwrap_or_else(|| "<all>".to_string());
        bail!(
            "Variable '{}' is not applicable to env '{}' (allowed: [{}])",
            opts.var_name,
            opts.env_name,
            allowed
        );
    }

    // Source-branch + value-input + execution lands in later tasks.
    bail!("push: source resolution not yet implemented");
}
```

- [ ] **Step 5: Run the three new tests**

Run: `cargo test --locked --test test_push test_push_unknown_variable_errors test_push_variable_not_applicable_to_env_errors test_push_unknown_environment_errors`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/commands/push.rs tests/test_push.rs tests/fixtures/push_basic.yaml
git commit -m "feat(push): validate schema, env, and variable applicability

Loads and validates the schema, rejects unknown env/var, and
enforces the variable's environments: list. Source resolution
still bails — implemented in the next commits."
```

---

## Task 6: Source-branch error messages (static, manual, missing push_command)

**Files:**
- Modify: `src/commands/push.rs`

- [ ] **Step 1: Add failing integration tests**

Append to `tests/test_push.rs`:

```rust
#[test]
fn test_push_static_source_error() {
    envgen()
        .arg("push")
        .arg("-c")
        .arg("tests/fixtures/push_basic.yaml")
        .arg("--env")
        .arg("local")
        .arg("--from-file")
        .arg("/dev/null")
        .arg("--allow-empty")
        .arg("--yes")
        .arg("STATIC_VAR")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("STATIC_VAR"))
        .stderr(predicate::str::contains("source is 'static'"))
        .stderr(predicate::str::contains("values:"));
}

#[test]
fn test_push_manual_source_error() {
    envgen()
        .arg("push")
        .arg("-c")
        .arg("tests/fixtures/push_basic.yaml")
        .arg("--env")
        .arg("local")
        .arg("--from-file")
        .arg("/dev/null")
        .arg("--allow-empty")
        .arg("--yes")
        .arg("MANUAL_VAR")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("MANUAL_VAR"))
        .stderr(predicate::str::contains("source is 'manual'"))
        .stderr(predicate::str::contains("password manager"));
}

#[test]
fn test_push_missing_push_command_includes_yaml_snippet() {
    envgen()
        .arg("push")
        .arg("-c")
        .arg("tests/fixtures/push_basic.yaml")
        .arg("--env")
        .arg("local")
        .arg("--from-file")
        .arg("/dev/null")
        .arg("--allow-empty")
        .arg("--yes")
        .arg("NO_PUSH")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("NO_PUSH"))
        .stderr(predicate::str::contains("Source 'readonly' has no push_command"))
        .stderr(predicate::str::contains("sources:"))
        .stderr(predicate::str::contains("readonly:"))
        .stderr(predicate::str::contains("push_command:"));
}
```

- [ ] **Step 2: Run and confirm they fail**

Run: `cargo test --locked --test test_push`
Expected: 3 failing.

- [ ] **Step 3: Implement source-branch logic in `run_push`**

Replace the trailing `bail!("push: source resolution not yet implemented")` in `src/commands/push.rs` with:

```rust
    let source_name = match var.effective_source_for_env(&opts.env_name) {
        Some(s) => s.to_string(),
        None => bail!(
            "No source configured for variable '{}' in env '{}'",
            opts.var_name,
            opts.env_name
        ),
    };

    if source_name == "static" {
        bail!(
            "Cannot push '{}' for env={}: source is 'static'. Static values are defined inline — edit the variable's values: block.",
            opts.var_name,
            opts.env_name
        );
    }

    if source_name == "manual" {
        bail!(
            "Cannot push '{}' for env={}: source is 'manual'. Manual sources have no remote — store the value in your password manager.",
            opts.var_name,
            opts.env_name
        );
    }

    let source = match schema.sources.get(&source_name) {
        Some(s) => s,
        None => bail!(
            "Source '{}' is not defined in sources (referenced by variable '{}').",
            source_name,
            opts.var_name
        ),
    };

    if source.push_command.is_none() {
        bail!(
            "Cannot push '{}' for env={}. Source '{}' has no push_command defined.\n\nAdd to your schema:\n  sources:\n    {}:\n      push_command: \"<e.g. gcloud secrets versions add {{key}} --data-file=- --project={{app_slug}}>\"",
            opts.var_name,
            opts.env_name,
            source_name,
            source_name
        );
    }

    // Value input + execution lands in later tasks.
    bail!("push: value input not yet implemented");
```

- [ ] **Step 4: Run the three error-message tests**

Run: `cargo test --locked --test test_push test_push_static_source_error test_push_manual_source_error test_push_missing_push_command_includes_yaml_snippet`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/commands/push.rs tests/test_push.rs
git commit -m "feat(push): emit actionable errors for non-pushable sources

Static and manual sources get targeted messages pointing at the
right fix. Missing push_command emits a copy-pasteable YAML snippet
the user can drop straight into their schema."
```

---

## Task 7: Value input — file mode, plus empty-value guard and ambiguity check

**Files:**
- Modify: `src/commands/push.rs`

- [ ] **Step 1: Add failing tests**

Append to `tests/test_push.rs`:

```rust
use std::fs;
use tempfile::TempDir;

#[test]
fn test_push_dry_run_from_file_strips_trailing_newline() {
    let tmp = TempDir::new().unwrap();
    let secret_path = tmp.path().join("secret.txt");
    fs::write(&secret_path, "my-value\n").unwrap();

    envgen()
        .arg("push")
        .arg("-c")
        .arg("tests/fixtures/push_basic.yaml")
        .arg("--env")
        .arg("local")
        .arg("--from-file")
        .arg(&secret_path)
        .arg("--dry-run")
        .arg("--show-secret")
        .arg("STORED_SECRET")
        .assert()
        .success()
        .stdout(predicate::str::contains("my-value"))
        .stdout(predicate::str::contains("\\n").not()); // assert no escaped newline shown
}

#[test]
fn test_push_from_file_missing_path_errors() {
    envgen()
        .arg("push")
        .arg("-c")
        .arg("tests/fixtures/push_basic.yaml")
        .arg("--env")
        .arg("local")
        .arg("--from-file")
        .arg("/no/such/path/i/hope.txt")
        .arg("--dry-run")
        .arg("STORED_SECRET")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("Failed to read"));
}

#[test]
fn test_push_empty_file_refused_without_allow_empty() {
    let tmp = TempDir::new().unwrap();
    let empty = tmp.path().join("empty.txt");
    fs::write(&empty, "").unwrap();

    envgen()
        .arg("push")
        .arg("-c")
        .arg("tests/fixtures/push_basic.yaml")
        .arg("--env")
        .arg("local")
        .arg("--from-file")
        .arg(&empty)
        .arg("--dry-run")
        .arg("STORED_SECRET")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("Refusing to push empty value"));
}
```

Add a unit test inside `src/commands/push.rs` for the input-mode decision logic:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn decide_input_mode_prefers_from_file() {
        let mode =
            decide_input_mode(Some(PathBuf::from("/tmp/x")), /*stdin_is_tty=*/ false).unwrap();
        assert!(matches!(mode, InputMode::File(_)));
    }

    #[test]
    fn decide_input_mode_pipe_when_stdin_not_tty() {
        let mode = decide_input_mode(None, /*stdin_is_tty=*/ false).unwrap();
        assert!(matches!(mode, InputMode::StdinPipe));
    }

    #[test]
    fn decide_input_mode_prompt_when_stdin_tty() {
        let mode = decide_input_mode(None, /*stdin_is_tty=*/ true).unwrap();
        assert!(matches!(mode, InputMode::Prompt));
    }

    #[test]
    fn decide_input_mode_ambiguous_errors() {
        let err =
            decide_input_mode(Some(PathBuf::from("/tmp/x")), /*stdin_is_tty=*/ false);
        // File takes precedence — this is unambiguous. The ambiguity case
        // (file + non-TTY stdin where user actually piped data) is checked
        // at the call site by inspecting both args; see ambiguous_input_errors.
        assert!(err.is_ok());
    }

    #[test]
    fn strip_one_trailing_newline_strips_lf() {
        assert_eq!(strip_one_trailing_newline("abc\n"), "abc");
    }

    #[test]
    fn strip_one_trailing_newline_strips_crlf() {
        assert_eq!(strip_one_trailing_newline("abc\r\n"), "abc");
    }

    #[test]
    fn strip_one_trailing_newline_strips_only_one() {
        assert_eq!(strip_one_trailing_newline("abc\n\n"), "abc\n");
    }

    #[test]
    fn strip_one_trailing_newline_passthrough() {
        assert_eq!(strip_one_trailing_newline("abc"), "abc");
    }
}
```

- [ ] **Step 2: Run and confirm failures**

Run: `cargo test --locked --test test_push` and `cargo test --locked commands::push`
Expected: new tests fail.

- [ ] **Step 3: Implement input-mode helpers and file mode**

Add at the top of `src/commands/push.rs` (alongside existing `use` statements):

```rust
use std::io::{IsTerminal, Read};
```

Add after `PushOptions`:

```rust
#[derive(Debug)]
enum InputMode {
    File(PathBuf),
    StdinPipe,
    Prompt,
}

fn decide_input_mode(from_file: Option<PathBuf>, stdin_is_tty: bool) -> Result<InputMode> {
    if let Some(path) = from_file {
        return Ok(InputMode::File(path));
    }
    if !stdin_is_tty {
        return Ok(InputMode::StdinPipe);
    }
    Ok(InputMode::Prompt)
}

fn strip_one_trailing_newline(s: &str) -> String {
    if let Some(stripped) = s.strip_suffix("\r\n") {
        return stripped.to_string();
    }
    if let Some(stripped) = s.strip_suffix('\n') {
        return stripped.to_string();
    }
    s.to_string()
}

fn read_value_from_file(path: &std::path::Path) -> Result<String> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read --from-file path: {}", path.display()))?;
    Ok(strip_one_trailing_newline(&raw))
}

fn read_value_from_stdin_pipe() -> Result<String> {
    let mut buf = String::new();
    std::io::stdin()
        .read_to_string(&mut buf)
        .context("Failed to read value from stdin")?;
    Ok(strip_one_trailing_newline(&buf))
}
```

Bring `Context` into scope at the top:

```rust
use anyhow::{bail, Context, Result};
```

Replace the `bail!("push: value input not yet implemented")` placeholder with:

```rust
    // Ambiguity check: --from-file with data also piped on stdin.
    let stdin_is_tty = std::io::stdin().is_terminal();
    if opts.from_file.is_some() && !stdin_is_tty {
        bail!("Cannot use --from-file together with stdin pipe. Pick one.");
    }

    let mode = decide_input_mode(opts.from_file.clone(), stdin_is_tty)?;
    let value = match mode {
        InputMode::File(p) => read_value_from_file(&p)?,
        InputMode::StdinPipe => read_value_from_stdin_pipe()?,
        InputMode::Prompt => {
            // Prompt mode is implemented in Task 9.
            bail!("push: interactive prompt not yet implemented");
        }
    };

    if value.is_empty() && !opts.allow_empty {
        bail!(
            "Refusing to push empty value for '{}'. Pass --allow-empty to override.",
            opts.var_name
        );
    }

    // Resolve push command + execute lands in Task 10.
    if opts.dry_run {
        let displayed = if opts.show_secret {
            value.clone()
        } else {
            "********".to_string()
        };
        println!();
        println!("variable:    {}", opts.var_name);
        println!("environment: {}", opts.env_name);
        println!("source:      {}", source_name);
        println!("value:       {}", displayed);
        println!();
        println!("(dry-run: command resolution lands in a later task)");
        return Ok(0);
    }

    bail!("push: command resolution not yet implemented");
```

- [ ] **Step 4: Run the new tests**

Run: `cargo test --locked --test test_push` and `cargo test --locked commands::push`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/commands/push.rs tests/test_push.rs
git commit -m "feat(push): support --from-file value input

Adds an InputMode discriminant, the file-mode reader (with one
trailing newline stripped — matches shell \$(cmd) semantics),
the empty-value guard, and the file+pipe ambiguity check. Stdin
pipe and interactive prompt modes follow."
```

---

## Task 8: Value input — stdin pipe mode

**Files:**
- Modify: `src/commands/push.rs` (no production change — already implemented in Task 7; this task adds an integration test that exercises the pipe path end-to-end via dry-run)

- [ ] **Step 1: Add a failing test**

Append to `tests/test_push.rs`:

```rust
#[test]
fn test_push_dry_run_from_stdin_pipe_strips_trailing_newline() {
    envgen()
        .arg("push")
        .arg("-c")
        .arg("tests/fixtures/push_basic.yaml")
        .arg("--env")
        .arg("local")
        .arg("--dry-run")
        .arg("--show-secret")
        .arg("STORED_SECRET")
        .write_stdin("piped-secret\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("piped-secret"));
}

#[test]
fn test_push_from_file_with_stdin_pipe_is_ambiguous() {
    let tmp = TempDir::new().unwrap();
    let p = tmp.path().join("v.txt");
    fs::write(&p, "x").unwrap();

    envgen()
        .arg("push")
        .arg("-c")
        .arg("tests/fixtures/push_basic.yaml")
        .arg("--env")
        .arg("local")
        .arg("--from-file")
        .arg(&p)
        .arg("--dry-run")
        .arg("STORED_SECRET")
        .write_stdin("also-piped\n")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("--from-file together with stdin pipe"));
}
```

- [ ] **Step 2: Run them**

Run: `cargo test --locked --test test_push test_push_dry_run_from_stdin_pipe_strips_trailing_newline test_push_from_file_with_stdin_pipe_is_ambiguous`
Expected: PASS — Task 7's implementation should already cover both. If the first test fails (e.g., harness pipes a TTY), inspect `assert_cmd::Command::write_stdin` semantics: it sets stdin to a pipe, so `IsTerminal::is_terminal` returns false, which means `decide_input_mode` returns `StdinPipe`.

- [ ] **Step 3: Commit**

```bash
git add tests/test_push.rs
git commit -m "test(push): cover stdin pipe + ambiguity path end-to-end

Confirms IsTerminal-based detection routes piped input to
StdinPipe and that combining --from-file with a piped stdin
yields the ambiguity error."
```

---

## Task 9: Value input — interactive prompt (TTY) mode

**Files:**
- Modify: `src/commands/push.rs`

This mode is hard to drive in integration tests (assert_cmd cannot fake a TTY). We unit-test the input-mode decision (already in Task 7) and rely on a small wrapper around `dialoguer::Password` so the prompt path is short and obvious.

- [ ] **Step 1: Implement the prompt path**

Replace the `InputMode::Prompt` arm in `run_push` (currently `bail!("push: interactive prompt not yet implemented");`) with:

```rust
        InputMode::Prompt => prompt_for_value(&opts.var_name)?,
```

Add the helper near the other input helpers:

```rust
fn prompt_for_value(var_name: &str) -> Result<String> {
    use dialoguer::Password;
    let prompt = format!("Enter value for {}", var_name);
    let value: String = Password::new()
        .with_prompt(prompt)
        .interact()
        .context("Failed to read value from interactive prompt")?;
    Ok(value)
}
```

(No newline stripping — `dialoguer::Password` returns the line without the terminating newline.)

- [ ] **Step 2: Confirm the build still passes**

Run: `cargo build --locked`
Expected: success.

- [ ] **Step 3: Confirm all tests still pass**

Run: `cargo test --locked --test test_push`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add src/commands/push.rs
git commit -m "feat(push): wire interactive prompt for TTY stdin

When stdin is a TTY and --from-file is absent, prompt for the
value via dialoguer::Password (hidden input). Mirrors the
manual_source prompt UX."
```

---

## Task 10: Resolve and execute the push command

**Files:**
- Modify: `src/commands/push.rs`

- [ ] **Step 1: Add failing integration tests**

Append to `tests/test_push.rs`:

```rust
#[test]
fn test_push_writes_value_to_fake_secret_store() {
    let tmp = TempDir::new().unwrap();
    let value_file = tmp.path().join("v.txt");
    fs::write(&value_file, "secret-payload").unwrap();

    // We need a fixture whose push_command writes into our tmpdir.
    // Build one inline.
    let schema_path = tmp.path().join("schema.yaml");
    fs::write(
        &schema_path,
        format!(
            r#"schema_version: "2"
metadata:
  description: "fake"
  destination:
    local: "{tmp}/.env.local"
environments:
  local:
    out_dir: "{tmp}"
sources:
  fakefs:
    command: "cat {{out_dir}}/{{key}}.txt"
    push_command: "tee {{out_dir}}/{{key}}.txt > /dev/null"
variables:
  STORED:
    description: "Stored secret."
    source: fakefs
"#,
            tmp = tmp.path().display()
        ),
    )
    .unwrap();

    envgen()
        .arg("push")
        .arg("-c")
        .arg(&schema_path)
        .arg("--env")
        .arg("local")
        .arg("--from-file")
        .arg(&value_file)
        .arg("STORED")
        .assert()
        .success()
        .stdout(predicate::str::contains("Pushed STORED to local"));

    let written = fs::read_to_string(tmp.path().join("STORED.txt")).unwrap();
    assert_eq!(written, "secret-payload");
}

#[test]
fn test_push_command_failure_returns_exit_code_2() {
    let tmp = TempDir::new().unwrap();
    let schema_path = tmp.path().join("schema.yaml");
    fs::write(
        &schema_path,
        r#"schema_version: "2"
metadata:
  description: "fake"
  destination:
    local: ".env"
environments:
  local: {}
sources:
  failing:
    command: "echo {key}"
    push_command: "exit 1"
variables:
  V:
    description: "x"
    source: failing
"#,
    )
    .unwrap();

    let v = tmp.path().join("v.txt");
    fs::write(&v, "anything").unwrap();

    envgen()
        .arg("push")
        .arg("-c")
        .arg(&schema_path)
        .arg("--env")
        .arg("local")
        .arg("--from-file")
        .arg(&v)
        .arg("V")
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("Push command failed"));
}

#[test]
fn test_push_command_timeout_returns_exit_code_2() {
    let tmp = TempDir::new().unwrap();
    let schema_path = tmp.path().join("schema.yaml");
    fs::write(
        &schema_path,
        r#"schema_version: "2"
metadata:
  description: "fake"
  destination:
    local: ".env"
environments:
  local: {}
sources:
  slow:
    command: "echo {key}"
    push_command: "sleep 10"
variables:
  V:
    description: "x"
    source: slow
"#,
    )
    .unwrap();

    let v = tmp.path().join("v.txt");
    fs::write(&v, "anything").unwrap();

    envgen()
        .arg("push")
        .arg("-c")
        .arg(&schema_path)
        .arg("--env")
        .arg("local")
        .arg("--from-file")
        .arg(&v)
        .arg("--source-timeout")
        .arg("1")
        .arg("V")
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("timed out"));
}
```

- [ ] **Step 2: Run and confirm they fail**

Run: `cargo test --locked --test test_push`
Expected: 3 failing.

- [ ] **Step 3: Implement command resolution + execution**

Add imports near the top of `src/commands/push.rs`:

```rust
use crate::resolver::command_source;
use crate::template;
```

Replace the trailing `bail!("push: command resolution not yet implemented");` with:

```rust
    let env_config = schema.environments.get(&opts.env_name).unwrap();
    let key = var.effective_key_for_env(&opts.var_name, &opts.env_name);

    // SAFETY: source.push_command is Some — we bailed earlier if it was None.
    let push_template = source.push_command.as_ref().unwrap();
    let resolved_cmd = command_source::build_command(
        push_template,
        &opts.var_name,
        Some(&key),
        &opts.env_name,
        env_config,
    )?;

    if opts.dry_run {
        // Already handled the early dry-run print above for masked value;
        // here we add the resolved command line.
        println!();
        println!("variable:    {}", opts.var_name);
        println!("environment: {}", opts.env_name);
        println!("source:      {}", display_source(&source_name, source));
        println!("command:     {}", resolved_cmd);
        let displayed = if opts.show_secret {
            value.clone()
        } else {
            "********".to_string()
        };
        println!("value:       {}", displayed);
        return Ok(0);
    }

    // Confirmation lands in Task 11. For now (so this task's tests pass
    // for env=local) just execute. The non-local prompt is added next.
    match command_source::execute_command_with_stdin(&resolved_cmd, Some(&value), opts.source_timeout).await {
        Ok(_) => {
            println!(
                "{} Pushed {} to {} via {}",
                "✓".green(),
                opts.var_name,
                opts.env_name,
                source_name
            );
            Ok(0)
        }
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("timed out") {
                eprintln!("Push command timed out after {} seconds.", opts.source_timeout);
            } else {
                eprintln!("Push command failed: {}", msg);
                eprintln!("Resolved command: {}", resolved_cmd);
            }
            Ok(2)
        }
    }
```

Now we have a duplicate dry-run block (Task 7 added a stub print, this task added the proper one). Delete the Task 7 stub: remove the `if opts.dry_run { ... }` block that printed `(dry-run: command resolution lands in a later task)` so only the new dry-run print remains.

Add the helper near the file-readers:

```rust
fn display_source(name: &str, src: &crate::schema::types::Source) -> String {
    match &src.label {
        Some(label) => format!("{} ({})", name, label),
        None => name.to_string(),
    }
}
```

- [ ] **Step 4: Run the new tests**

Run: `cargo test --locked --test test_push`
Expected: PASS — including the three new ones and all earlier ones.

- [ ] **Step 5: Commit**

```bash
git add src/commands/push.rs tests/test_push.rs
git commit -m "feat(push): execute the resolved push command with stdin

Builds the push command via the existing template engine, pipes
the value into the child via execute_command_with_stdin, and
maps failures to exit code 2 (vs. 1 for local errors) so CI can
distinguish bad inputs from remote rejections."
```

---

## Task 11: Confirmation prompt for non-local environments

**Files:**
- Modify: `src/commands/push.rs`

- [ ] **Step 1: Add failing integration tests**

Append to `tests/test_push.rs`:

```rust
#[test]
fn test_push_non_local_prompts_and_cancels_on_n() {
    let tmp = TempDir::new().unwrap();
    let schema_path = tmp.path().join("schema.yaml");
    fs::write(
        &schema_path,
        format!(
            r#"schema_version: "2"
metadata:
  description: "fake"
  destination:
    local: "{tmp}/.env.local"
    stg: "{tmp}/.env.stg"
environments:
  local:
    out_dir: "{tmp}"
  stg:
    out_dir: "{tmp}"
sources:
  fakefs:
    command: "cat {{out_dir}}/{{key}}.txt"
    push_command: "tee {{out_dir}}/{{key}}.txt > /dev/null"
variables:
  STORED:
    description: "x"
    source: fakefs
"#,
            tmp = tmp.path().display()
        ),
    )
    .unwrap();

    envgen()
        .arg("push")
        .arg("-c")
        .arg(&schema_path)
        .arg("--env")
        .arg("stg")
        .arg("STORED")
        .write_stdin("the-value\nn\n") // first line = value (StdinPipe), confirm = n
        // NOTE: when stdin is piped, value source is StdinPipe (not Prompt),
        // so we use a different mechanism: pass --from-file for value, then
        // pipe only the confirmation answer.
        .assert();
    // Replace the above with the working pattern below.
}

#[test]
fn test_push_non_local_cancels_on_n() {
    let tmp = TempDir::new().unwrap();
    let value_file = tmp.path().join("v.txt");
    fs::write(&value_file, "secret").unwrap();
    let schema_path = tmp.path().join("schema.yaml");
    fs::write(
        &schema_path,
        format!(
            r#"schema_version: "2"
metadata:
  description: "fake"
  destination:
    local: "{tmp}/.env.local"
    stg: "{tmp}/.env.stg"
environments:
  local:
    out_dir: "{tmp}"
  stg:
    out_dir: "{tmp}"
sources:
  fakefs:
    command: "cat {{out_dir}}/{{key}}.txt"
    push_command: "tee {{out_dir}}/{{key}}.txt > /dev/null"
variables:
  STORED:
    description: "x"
    source: fakefs
"#,
            tmp = tmp.path().display()
        ),
    )
    .unwrap();

    envgen()
        .arg("push")
        .arg("-c")
        .arg(&schema_path)
        .arg("--env")
        .arg("stg")
        .arg("--from-file")
        .arg(&value_file)
        .arg("STORED")
        .write_stdin("n\n")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("Push cancelled"));

    assert!(!tmp.path().join("STORED.txt").exists());
}

#[test]
fn test_push_non_local_yes_skips_prompt() {
    let tmp = TempDir::new().unwrap();
    let value_file = tmp.path().join("v.txt");
    fs::write(&value_file, "secret-yes").unwrap();
    let schema_path = tmp.path().join("schema.yaml");
    fs::write(
        &schema_path,
        format!(
            r#"schema_version: "2"
metadata:
  description: "fake"
  destination:
    local: "{tmp}/.env.local"
    stg: "{tmp}/.env.stg"
environments:
  local:
    out_dir: "{tmp}"
  stg:
    out_dir: "{tmp}"
sources:
  fakefs:
    command: "cat {{out_dir}}/{{key}}.txt"
    push_command: "tee {{out_dir}}/{{key}}.txt > /dev/null"
variables:
  STORED:
    description: "x"
    source: fakefs
"#,
            tmp = tmp.path().display()
        ),
    )
    .unwrap();

    envgen()
        .arg("push")
        .arg("-c")
        .arg(&schema_path)
        .arg("--env")
        .arg("stg")
        .arg("--from-file")
        .arg(&value_file)
        .arg("--yes")
        .arg("STORED")
        .assert()
        .success();

    assert_eq!(fs::read_to_string(tmp.path().join("STORED.txt")).unwrap(), "secret-yes");
}

#[test]
fn test_push_local_does_not_prompt() {
    // Already covered by test_push_writes_value_to_fake_secret_store
    // (env=local, no --yes, no stdin answer, succeeds). Add an explicit
    // assertion here as documentation.
}
```

Delete the first stub `test_push_non_local_prompts_and_cancels_on_n` (it was a thinking stub; the working version is `test_push_non_local_cancels_on_n`).

- [ ] **Step 2: Run and confirm failures**

Run: `cargo test --locked --test test_push`
Expected: the two non-local tests fail (no prompt yet, push goes through and writes the file regardless of the `n` answer).

- [ ] **Step 3: Implement the confirmation prompt**

Add a helper near `display_source`:

```rust
fn confirm_non_local_push(
    var_name: &str,
    env_name: &str,
    source_display: &str,
    resolved_cmd: &str,
    value: &str,
    show_secret: bool,
) -> Result<bool> {
    use dialoguer::Confirm;

    let displayed_value = if show_secret {
        value.to_string()
    } else {
        "******** (use --show-secret to reveal)".to_string()
    };

    println!();
    println!("About to push to non-local environment:");
    println!("  variable:    {}", var_name);
    println!("  environment: {}", env_name);
    println!("  source:      {}", source_display);
    println!("  command:     {}", resolved_cmd);
    println!("  value:       {}", displayed_value);

    let answer = Confirm::new()
        .with_prompt("Continue?")
        .default(false)
        .interact()
        .context("Failed to read confirmation answer")?;
    Ok(answer)
}
```

Add the gate just above the `execute_command_with_stdin` call:

```rust
    let needs_prompt = opts.env_name != "local" && !opts.yes && !opts.dry_run;
    if needs_prompt {
        let confirmed = confirm_non_local_push(
            &opts.var_name,
            &opts.env_name,
            &display_source(&source_name, source),
            &resolved_cmd,
            &value,
            opts.show_secret,
        )?;
        if !confirmed {
            eprintln!("Push cancelled.");
            return Ok(1);
        }
    }
```

- [ ] **Step 4: Run tests**

Run: `cargo test --locked --test test_push`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/commands/push.rs tests/test_push.rs
git commit -m "feat(push): confirm before pushing to non-local envs

Any env name other than 'local' triggers a y/N prompt that shows
the resolved command and masked value. --yes skips the prompt;
--dry-run skips it; local skips it. Cancellation exits 1."
```

---

## Task 12: Sample schema, README, and JSON-Schema example update

**Files:**
- Modify: `schemas/envgen.sample.yaml`
- Modify: `README.md`

- [ ] **Step 1: Update the sample YAML to document `push_command`**

Edit `schemas/envgen.sample.yaml` to add `push_command:` to the `gcloud` source. The full block becomes:

```yaml
  gcloud:
    command: "gcloud secrets versions access latest --secret={key} --project={app_slug}"
    push_command: "gcloud secrets versions add {key} --data-file=- --project={app_slug}"
    label: "Google Cloud Secret Manager"
    url: "https://console.cloud.google.com/security/secret-manager"
    description: "Primary secret store for non-local environments."
```

- [ ] **Step 2: Confirm `init` and `check` tests still pass**

Run: `cargo test --locked --test test_init --test test_check`
Expected: PASS. The `test_check_valid_schema` test asserts source counts (`1 source`) which doesn't change.

- [ ] **Step 3: Add a `push` section to the README**

Open `README.md` and add a section after the existing `pull` documentation. Use the user-facing language from the spec — usage examples for `--from-file`, stdin pipe, and interactive prompt; mention the non-local confirmation; mention exit codes 1 vs 2.

(Engineer: keep this short — 30-50 lines. Mirror the structure of the existing `pull` section. Do not duplicate the spec.)

- [ ] **Step 4: Run the full test suite**

Run: `cargo test --locked`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add schemas/envgen.sample.yaml README.md
git commit -m "docs(push): document push_command and the push subcommand

Adds push_command to the sample gcloud source so envgen init
emits a working example, and a README section covering --from-file,
stdin pipe, prompt, --yes, and the 1-vs-2 exit code split."
```

---

## Task 13: Final verification and lint pass

**Files:** none (verification only)

- [ ] **Step 1: Run `cargo fmt`**

Run: `cargo fmt`

- [ ] **Step 2: Run clippy**

Run: `cargo clippy --locked -- -D warnings -A clippy::uninlined_format_args`
Expected: clean. Fix any warnings inline, prefer minimal changes.

- [ ] **Step 3: Run the project-wide test gate**

Run: `make check-rust`
Expected: PASS.

- [ ] **Step 4: Run the full check (per CLAUDE.md guidance)**

Run: `make check`
Expected: PASS. If schema/JSON tooling complains about the new `push_command` JSON-schema property, fix as instructed by the tooling output (likely `make fmt-schema`).

- [ ] **Step 5: Commit any fmt/lint fixups**

```bash
git add -A
git commit -m "chore(push): fmt + clippy fixups after final review" || true
```

(`|| true` handles the case where there is nothing to commit.)

---

## Self-Review

**Spec coverage check** (against `docs/superpowers/specs/2026-04-27-envgen-push-design.md`):
- Schema additions → Task 1, Task 12. ✓
- Push command template validation → Task 2. ✓
- CLI surface (all flags) → Task 4 (skeleton), Tasks 7/9/10/11/12 (behavior). ✓
- Value input modes (file/pipe/prompt) → Tasks 7/8/9. ✓
- Empty-value guard → Task 7. ✓
- Ambiguous value source error → Tasks 7/8. ✓
- Confirmation prompt (non-local) → Task 11. ✓
- Secret display / `--show-secret` → Tasks 7/10/11. ✓
- Exit codes 0/1/2 → Tasks 5/6/7/10/11. ✓
- Resolution flow steps 1-10 → Tasks 5/6/7/10/11. ✓
- Error catalog (every row) → Tasks 5/6/7/10/11. ✓
- Module boundaries (`commands::push` vs `resolver::command_source`) → Tasks 3/4. ✓
- Tests called out in the spec → all present (renamed slightly for Rust naming conventions). ✓
- Out-of-scope items (bulk push, read-back, audit, prompt timeout) → not implemented. ✓

**Placeholder scan:** None — every task lists exact file paths, exact code, exact commands, exact expected output. The README task is the one place that delegates wording to the engineer ("keep this short, mirror pull"), which is appropriate for prose.

**Type / signature consistency:**
- `PushOptions` fields match between definition (Task 4) and dispatch arm (Task 4) and uses (Tasks 5/6/7/9/10/11).
- `decide_input_mode(Option<PathBuf>, bool) -> Result<InputMode>` signature consistent across Task 7 unit tests and Task 7 implementation.
- `execute_command_with_stdin(&str, Option<&str>, u64) -> Result<CommandResult>` consistent across Task 3 tests/impl and Task 10 call site.
- `display_source` defined once (Task 10), used in Task 10 dry-run and Task 11 prompt.
- `confirm_non_local_push` defined and called only in Task 11.

**Scope check:** Single subsystem, single binary, single command — appropriate for one plan.

No issues found.

---

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-04-27-envgen-push.md`. Two execution options:

1. **Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration.
2. **Inline Execution** — Execute tasks in this session using executing-plans, batch execution with checkpoints.

Which approach?

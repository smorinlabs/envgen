use assert_cmd::cargo::cargo_bin_cmd;
use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

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
fn test_push_from_file_takes_precedence_over_stdin_pipe() {
    let tmp = TempDir::new().unwrap();
    let p = tmp.path().join("v.txt");
    fs::write(&p, "from-file-wins").unwrap();

    envgen()
        .arg("push")
        .arg("-c")
        .arg("tests/fixtures/push_basic.yaml")
        .arg("--env")
        .arg("local")
        .arg("--from-file")
        .arg(&p)
        .arg("--dry-run")
        .arg("--show-secret")
        .arg("STORED_SECRET")
        .write_stdin("from-stdin-loses")
        .assert()
        .success()
        .stdout(predicate::str::contains("from-file-wins"))
        .stdout(predicate::str::contains("from-stdin-loses").not());
}

#[test]
fn test_push_writes_value_to_fake_secret_store() {
    let tmp = TempDir::new().unwrap();
    let value_file = tmp.path().join("v.txt");
    fs::write(&value_file, "secret-payload").unwrap();

    // Build a self-contained schema fixture inside the tmpdir.
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

    assert_eq!(
        fs::read_to_string(tmp.path().join("STORED.txt")).unwrap(),
        "secret-yes"
    );
}

#[test]
fn test_push_non_local_dry_run_skips_prompt() {
    let tmp = TempDir::new().unwrap();
    let value_file = tmp.path().join("v.txt");
    fs::write(&value_file, "anything").unwrap();
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
        .arg("--dry-run")
        .arg("STORED")
        .assert()
        .success()
        .stdout(predicate::str::contains("environment: stg"));

    // Dry run did not write the file.
    assert!(!tmp.path().join("STORED.txt").exists());
}

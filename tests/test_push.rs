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

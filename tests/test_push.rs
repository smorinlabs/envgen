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

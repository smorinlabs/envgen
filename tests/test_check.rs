use assert_cmd::cargo::cargo_bin_cmd;
use assert_cmd::Command;
use predicates::prelude::*;

fn envgen() -> Command {
    cargo_bin_cmd!("envgen")
}

#[test]
fn test_check_valid_schema() {
    envgen()
        .arg("check")
        .arg("-c")
        .arg("tests/fixtures/valid_frontend.yaml")
        .assert()
        .success()
        .stdout(predicate::str::contains("Schema valid"))
        .stdout(predicate::str::contains("5 variables"))
        .stdout(predicate::str::contains("3 environments"))
        .stdout(predicate::str::contains("1 source"));
}

#[test]
fn test_check_valid_backend_schema() {
    envgen()
        .arg("check")
        .arg("-c")
        .arg("tests/fixtures/valid_backend.yaml")
        .assert()
        .success()
        .stdout(predicate::str::contains("Schema valid"));
}

#[test]
fn test_check_invalid_schema() {
    envgen()
        .arg("check")
        .arg("-c")
        .arg("tests/fixtures/invalid_schema.yaml")
        .assert()
        .failure()
        .stdout(predicate::str::contains("Schema errors"))
        .stdout(predicate::str::contains("STATIC_NO_VALUES"))
        .stdout(predicate::str::contains("missing required property"));
}

#[test]
fn test_check_missing_file() {
    envgen()
        .arg("check")
        .arg("-c")
        .arg("tests/fixtures/does_not_exist.yaml")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Failed to read schema file"));
}

#[test]
fn test_check_no_schema_flag() {
    envgen()
        .arg("check")
        .assert()
        .failure()
        .stderr(predicate::str::contains("--config"));
}

#[test]
fn test_check_semantic_validation_runs_after_structural() {
    envgen()
        .arg("check")
        .arg("-c")
        .arg("tests/fixtures/semantic_invalid_schema.yaml")
        .assert()
        .failure()
        .stdout(predicate::str::contains("Schema errors"))
        .stdout(predicate::str::contains("nonexistent-source"))
        .stdout(predicate::str::contains("not defined in sources"));
}

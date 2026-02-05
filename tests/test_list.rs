use assert_cmd::cargo::cargo_bin_cmd;
use assert_cmd::Command;
use predicates::prelude::*;

fn envgen() -> Command {
    cargo_bin_cmd!("envgen")
}

#[test]
fn test_list_table_output() {
    envgen()
        .arg("list")
        .arg("-c")
        .arg("tests/fixtures/valid_frontend.yaml")
        .assert()
        .success()
        .stdout(predicate::str::contains("VITE_ENV"))
        .stdout(predicate::str::contains("VITE_BASE_URL"))
        .stdout(predicate::str::contains("VITE_API_KEY"))
        .stdout(predicate::str::contains("6 variables"));
}

#[test]
fn test_list_with_env_filter() {
    envgen()
        .arg("list")
        .arg("-c")
        .arg("tests/fixtures/valid_backend.yaml")
        .arg("-e")
        .arg("local")
        .assert()
        .success()
        .stdout(predicate::str::contains("GOOGLE_CLIENT_ID"))
        .stdout(predicate::str::contains("4 variables")); // OPTIONAL_KEY is staging+production only
}

#[test]
fn test_list_json_format() {
    envgen()
        .arg("list")
        .arg("-c")
        .arg("tests/fixtures/valid_frontend.yaml")
        .arg("--format")
        .arg("json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"name\""))
        .stdout(predicate::str::contains("VITE_ENV"))
        .stdout(predicate::str::contains("\"source\""));
}

#[test]
fn test_list_invalid_env() {
    envgen()
        .arg("list")
        .arg("-c")
        .arg("tests/fixtures/valid_frontend.yaml")
        .arg("-e")
        .arg("nonexistent")
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn test_list_invalid_format() {
    envgen()
        .arg("list")
        .arg("-c")
        .arg("tests/fixtures/valid_frontend.yaml")
        .arg("--format")
        .arg("csv")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unknown format"));
}

#[test]
fn test_list_invalid_schema_fails() {
    envgen()
        .arg("list")
        .arg("-c")
        .arg("tests/fixtures/invalid_schema.yaml")
        .assert()
        .failure()
        .stdout(predicate::str::contains("Schema errors"))
        .stdout(predicate::str::contains("STATIC_NO_VALUES"));
}

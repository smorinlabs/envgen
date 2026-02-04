use assert_cmd::cargo::cargo_bin_cmd;
use assert_cmd::Command;
use predicates::prelude::*;

fn envgen() -> Command {
    cargo_bin_cmd!("envgen")
}

#[test]
fn test_docs_markdown_output() {
    envgen()
        .arg("docs")
        .arg("-c")
        .arg("tests/fixtures/valid_frontend.yaml")
        .assert()
        .success()
        .stdout(predicate::str::contains("# envgen schema documentation"))
        .stdout(predicate::str::contains("### `VITE_ENV`"))
        .stdout(predicate::str::contains("### `VITE_API_KEY`"));
}

#[test]
fn test_docs_with_env_filter() {
    envgen()
        .arg("docs")
        .arg("-c")
        .arg("tests/fixtures/valid_backend.yaml")
        .arg("-e")
        .arg("local")
        .assert()
        .success()
        .stdout(predicate::str::contains("- Environment: `local`"))
        .stdout(predicate::str::contains("### `TOKEN_ENCRYPTION_KEY`"))
        .stdout(predicate::str::contains("#### Notes"));
}

#[test]
fn test_docs_invalid_env() {
    envgen()
        .arg("docs")
        .arg("-c")
        .arg("tests/fixtures/valid_frontend.yaml")
        .arg("-e")
        .arg("nonexistent")
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn test_docs_invalid_schema() {
    envgen()
        .arg("docs")
        .arg("-c")
        .arg("tests/fixtures/invalid_schema.yaml")
        .assert()
        .failure()
        .stdout(predicate::str::contains("Schema errors"))
        .stderr(predicate::str::contains("Schema validation failed"));
}

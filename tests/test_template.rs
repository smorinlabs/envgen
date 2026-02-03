use assert_cmd::Command;
use predicates::prelude::*;

fn envtool() -> Command {
    Command::cargo_bin("envtool").unwrap()
}

#[test]
fn test_template_expansion_in_dry_run() {
    // The dry-run output should show resolved commands with placeholders expanded
    envtool()
        .arg("pull")
        .arg("-s")
        .arg("tests/fixtures/valid_frontend.yaml")
        .arg("-e")
        .arg("local")
        .arg("--dry-run")
        .assert()
        .success()
        // The echo-source command should have {key} expanded to API_KEY
        .stdout(predicate::str::contains("echo API_KEY-local"));
}

#[test]
fn test_template_expansion_staging() {
    envtool()
        .arg("pull")
        .arg("-s")
        .arg("tests/fixtures/valid_frontend.yaml")
        .arg("-e")
        .arg("staging")
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains("echo API_KEY-staging"));
}

#[test]
fn test_static_template_expansion_in_dry_run() {
    // Static values with {placeholder} references should be shown expanded
    envtool()
        .arg("pull")
        .arg("-s")
        .arg("tests/fixtures/valid_frontend.yaml")
        .arg("-e")
        .arg("production")
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains("VITE_BASE_URL"))
        .stdout(predicate::str::contains("VITE_PROJECT_ID"));
}

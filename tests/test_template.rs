use assert_cmd::cargo::cargo_bin_cmd;
use assert_cmd::Command;
use predicates::prelude::*;

fn envgen() -> Command {
    cargo_bin_cmd!("envgen")
}

#[test]
fn test_template_expansion_in_dry_run() {
    // The dry-run output should show the effective resolver per environment.
    envgen()
        .arg("pull")
        .arg("-s")
        .arg("tests/fixtures/valid_frontend.yaml")
        .arg("-e")
        .arg("local")
        .arg("--dry-run")
        .assert()
        .success()
        // For local, VITE_API_KEY is static (schema v2 resolvers)
        .stdout(predicate::str::contains("VITE_API_KEY\n    source:  static"))
        // Sensitive values are masked by default
        .stdout(predicate::str::contains("value:   API_..."));
}

#[test]
fn test_template_expansion_staging() {
    envgen()
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
    envgen()
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

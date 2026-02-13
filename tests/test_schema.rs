use assert_cmd::cargo::cargo_bin_cmd;
use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

fn envgen() -> Command {
    cargo_bin_cmd!("envgen")
}

fn schema_filename() -> String {
    format!(
        "envgen.schema.v{}.json",
        env!("ENVGEN_SCHEMA_ARTIFACT_VERSION")
    )
}

fn fixture_content() -> String {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("schemas");
    path.push(schema_filename());
    fs::read_to_string(path).unwrap()
}

#[test]
fn test_schema_stdout_prints_schema() {
    let schema_artifact_version = env!("ENVGEN_SCHEMA_ARTIFACT_VERSION");
    envgen()
        .arg("schema")
        .arg("--output")
        .arg("-")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"$schema\""))
        .stdout(predicate::str::contains("envgen YAML schema"))
        .stdout(predicate::str::contains("\"x-envgen-schema-version\""))
        .stdout(predicate::str::contains(schema_artifact_version));
}

#[test]
fn test_schema_default_output_path() {
    let tmp = TempDir::new().unwrap();
    let default_path = tmp.path().join(schema_filename());

    envgen()
        .current_dir(tmp.path())
        .arg("schema")
        .assert()
        .success()
        .stdout(predicate::str::contains("Wrote JSON Schema"));

    let content = fs::read_to_string(default_path).unwrap();
    assert_eq!(content, fixture_content());
}

#[test]
fn test_schema_output_file() {
    let tmp = TempDir::new().unwrap();
    let output_path = tmp.path().join("custom.schema.json");

    envgen()
        .arg("schema")
        .arg("--output")
        .arg(output_path.to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("Wrote JSON Schema"));

    let content = fs::read_to_string(output_path).unwrap();
    assert_eq!(content, fixture_content());
}

#[test]
fn test_schema_creates_parent_directories() {
    let tmp = TempDir::new().unwrap();
    let output_path = tmp.path().join("schemas/nested/custom.schema.json");

    envgen()
        .arg("schema")
        .arg("--output")
        .arg(output_path.to_str().unwrap())
        .assert()
        .success();

    let content = fs::read_to_string(output_path).unwrap();
    assert_eq!(content, fixture_content());
}

#[test]
fn test_schema_output_directory() {
    let tmp = TempDir::new().unwrap();
    let output_dir = tmp.path().join("schemas");
    fs::create_dir(&output_dir).unwrap();

    envgen()
        .arg("schema")
        .arg("-o")
        .arg(output_dir.to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("Wrote JSON Schema"));

    let output_path = output_dir.join(schema_filename());
    let content = fs::read_to_string(output_path).unwrap();
    assert_eq!(content, fixture_content());
}

#[test]
fn test_schema_refuses_overwrite_without_force() {
    let tmp = TempDir::new().unwrap();
    let output_path = tmp.path().join(schema_filename());
    fs::write(&output_path, "existing content").unwrap();

    envgen()
        .current_dir(tmp.path())
        .arg("schema")
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
}

#[test]
fn test_schema_force_overwrites() {
    let tmp = TempDir::new().unwrap();
    let output_path = tmp.path().join(schema_filename());
    fs::write(&output_path, "existing content").unwrap();

    envgen()
        .current_dir(tmp.path())
        .arg("schema")
        .arg("--force")
        .assert()
        .success()
        .stdout(predicate::str::contains("Wrote JSON Schema"));

    let content = fs::read_to_string(output_path).unwrap();
    assert_eq!(content, fixture_content());
}

#[test]
fn test_schema_quiet_suppresses_output() {
    let tmp = TempDir::new().unwrap();
    let output_path = tmp.path().join("quiet.schema.json");

    envgen()
        .arg("schema")
        .arg("--output")
        .arg(output_path.to_str().unwrap())
        .arg("--quiet")
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

use assert_cmd::cargo::cargo_bin_cmd;
use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

fn envgen() -> Command {
    cargo_bin_cmd!("envgen")
}

fn fixture_content() -> String {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("schemas/envgen.sample.yaml");
    fs::read_to_string(path).unwrap()
}

#[test]
fn test_init_default_output_path() {
    let tmp = TempDir::new().unwrap();
    let default_path = tmp.path().join("env.dev.yaml");

    envgen()
        .current_dir(tmp.path())
        .arg("init")
        .assert()
        .success()
        .stdout(predicate::str::contains("Wrote sample schema"));

    let content = fs::read_to_string(default_path).unwrap();
    assert_eq!(content, fixture_content());
}

#[test]
fn test_init_output_file() {
    let tmp = TempDir::new().unwrap();
    let output_path = tmp.path().join("custom.yaml");

    envgen()
        .arg("init")
        .arg("--output")
        .arg(output_path.to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("Wrote sample schema"));

    let content = fs::read_to_string(output_path).unwrap();
    assert_eq!(content, fixture_content());
}

#[test]
fn test_init_creates_parent_directories() {
    let tmp = TempDir::new().unwrap();
    let output_path = tmp.path().join("configs/nested/env.dev.yaml");

    envgen()
        .arg("init")
        .arg("--output")
        .arg(output_path.to_str().unwrap())
        .assert()
        .success();

    let content = fs::read_to_string(output_path).unwrap();
    assert_eq!(content, fixture_content());
}

#[test]
fn test_init_output_directory() {
    let tmp = TempDir::new().unwrap();
    let output_dir = tmp.path().join("configs");
    fs::create_dir(&output_dir).unwrap();

    envgen()
        .arg("init")
        .arg("-o")
        .arg(output_dir.to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("Wrote sample schema"));

    let output_path = output_dir.join("env.dev.yaml");
    let content = fs::read_to_string(output_path).unwrap();
    assert_eq!(content, fixture_content());
}

#[test]
fn test_init_refuses_overwrite_without_force() {
    let tmp = TempDir::new().unwrap();
    let output_path = tmp.path().join("env.dev.yaml");
    fs::write(&output_path, "existing content").unwrap();

    envgen()
        .current_dir(tmp.path())
        .arg("init")
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
}

#[test]
fn test_init_force_overwrites() {
    let tmp = TempDir::new().unwrap();
    let output_path = tmp.path().join("env.dev.yaml");
    fs::write(&output_path, "existing content").unwrap();

    envgen()
        .current_dir(tmp.path())
        .arg("init")
        .arg("--force")
        .assert()
        .success()
        .stdout(predicate::str::contains("Wrote sample schema"));

    let content = fs::read_to_string(output_path).unwrap();
    assert_eq!(content, fixture_content());
}

#[test]
fn test_init_quiet_suppresses_output() {
    let tmp = TempDir::new().unwrap();
    let output_path = tmp.path().join("quiet.yaml");

    envgen()
        .arg("init")
        .arg("--output")
        .arg(output_path.to_str().unwrap())
        .arg("--quiet")
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

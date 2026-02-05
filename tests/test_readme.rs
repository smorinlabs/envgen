use assert_cmd::cargo::cargo_bin_cmd;
use assert_cmd::Command;
use std::fs;
use std::path::PathBuf;

fn envgen() -> Command {
    cargo_bin_cmd!("envgen")
}

fn fixture_content() -> String {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("README.md");
    fs::read_to_string(path).unwrap()
}

#[test]
fn test_readme_prints_embedded_readme() {
    let assert = envgen().arg("readme").assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    assert_eq!(stdout, fixture_content());
}

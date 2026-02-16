use std::fs;

fn main() {
    println!("cargo:rerun-if-changed=SCHEMA_VERSION");
    println!("cargo:rerun-if-changed=build.rs");

    let schema_version = fs::read_to_string("SCHEMA_VERSION")
        .expect("failed to read SCHEMA_VERSION")
        .trim()
        .to_string();

    if !is_strict_semver(&schema_version) {
        panic!("SCHEMA_VERSION must be strict semver X.Y.Z, got: {schema_version}");
    }

    println!("cargo:rustc-env=ENVGEN_SCHEMA_ARTIFACT_VERSION={schema_version}");
}

fn is_strict_semver(version: &str) -> bool {
    let mut parts = version.split('.');
    let first = parts.next();
    let second = parts.next();
    let third = parts.next();

    if parts.next().is_some() {
        return false;
    }

    match (first, second, third) {
        (Some(a), Some(b), Some(c)) => [a, b, c].iter().all(|part| {
            !part.is_empty()
                && part.chars().all(|ch| ch.is_ascii_digit())
                && !(part.len() > 1 && part.starts_with('0'))
        }),
        _ => false,
    }
}

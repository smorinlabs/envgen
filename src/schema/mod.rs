pub mod parser;
pub mod structural;
pub mod types;
pub mod validation;
pub mod validator;

pub const JSON_SCHEMA_FILENAME: &str =
    concat!("envgen.schema.v", env!("CARGO_PKG_VERSION"), ".json");

pub const JSON_SCHEMA: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/schemas/envgen.schema.v",
    env!("CARGO_PKG_VERSION"),
    ".json"
));

#[cfg(test)]
mod tests {
    use super::JSON_SCHEMA;
    use serde_json::Value;
    #[cfg(target_os = "linux")]
    use std::io::Write;
    #[cfg(target_os = "linux")]
    use std::process::Command;

    const EXPECTED_SCHEMA_DRAFT: &str = "https://json-schema.org/draft/2020-12/schema";

    #[test]
    fn embedded_schema_is_valid_json() {
        let result: Result<Value, _> = serde_json::from_str(JSON_SCHEMA);
        assert!(
            result.is_ok(),
            "schema.json is not valid JSON: {}",
            result.unwrap_err()
        );
    }

    #[test]
    fn embedded_schema_declares_draft_2020_12() {
        let schema: Value = serde_json::from_str(JSON_SCHEMA).unwrap();
        assert_eq!(
            schema.get("$schema").and_then(|v| v.as_str()),
            Some(EXPECTED_SCHEMA_DRAFT),
            "Schema must declare Draft 2020-12 via $schema field"
        );
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn embedded_schema_is_valid_draft_2020_12() {
        let uvx = "uvx";

        if Command::new(uvx).arg("--version").output().is_err() {
            eprintln!("Skipping Draft 2020-12 meta-validation: `uvx` not found in PATH");
            return;
        }

        let mut tmp = tempfile::NamedTempFile::new().expect("create temp schema file");
        tmp.write_all(JSON_SCHEMA.as_bytes())
            .expect("write embedded schema to temp file");
        tmp.flush().expect("flush temp schema file");

        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let uv_cache_dir = format!("{}/.uv-cache", manifest_dir);
        let uv_tool_dir = format!("{}/.uv-tools", manifest_dir);

        let output = Command::new(uvx)
            .env("UV_CACHE_DIR", uv_cache_dir)
            .env("UV_TOOL_DIR", uv_tool_dir)
            .arg("check-jsonschema")
            .arg("--check-metaschema")
            .arg(tmp.path())
            .output()
            .expect(
                "failed to run `uvx check-jsonschema`; install uv to enable schema meta-validation",
            );

        if !output.status.success() {
            panic!(
                "Schema failed Draft 2020-12 meta-validation:\nstdout:\n{}\nstderr:\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
}

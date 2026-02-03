use anyhow::{Context, Result};
use std::path::Path;

use super::types::Schema;

/// Parse a YAML schema file from the given path.
pub fn parse_schema_file(path: &Path) -> Result<Schema> {
    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read schema file: {}", path.display()))?;
    parse_schema(&contents)
}

/// Parse a YAML schema from a string.
pub fn parse_schema(yaml: &str) -> Result<Schema> {
    let schema: Schema = serde_yaml::from_str(yaml).context("Failed to parse schema YAML")?;
    Ok(schema)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_schema() {
        let yaml = r#"
schema_version: "1"
metadata:
  description: "Test schema"
  destination:
    local: ".env"
environments:
  local:
    project: "test-project"
sources:
  test-source:
    command: "echo {key}"
variables:
  MY_VAR:
    description: "A test variable"
    sensitive: false
    source: static
    values:
      local: "hello"
"#;
        let schema = parse_schema(yaml).unwrap();
        assert_eq!(schema.schema_version, "1");
        assert_eq!(schema.variables.len(), 1);
        assert!(schema.variables.contains_key("MY_VAR"));
    }

    #[test]
    fn test_parse_invalid_yaml() {
        let yaml = "not: valid: yaml: [";
        assert!(parse_schema(yaml).is_err());
    }
}

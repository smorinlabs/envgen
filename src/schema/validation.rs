use anyhow::{Context, Result};
use std::path::Path;

use super::parser::parse_schema_file;
use super::structural;
use super::types::Schema;
use super::validator::validate_schema as validate_semantic_schema;

pub enum SchemaValidation {
    Valid(Schema),
    Invalid(Vec<String>),
}

pub fn load_and_validate_schema_file(path: &Path) -> Result<SchemaValidation> {
    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read schema file: {}", path.display()))?;
    let yaml_value: serde_yaml::Value =
        serde_yaml::from_str(&contents).context("Failed to parse schema YAML")?;

    let instance_json: serde_json::Value = serde_json::to_value(&yaml_value)
        .context("Failed to convert schema YAML to JSON for validation")?;

    let structural_errors = structural::validate_instance(&instance_json)?;
    if !structural_errors.is_empty() {
        return Ok(SchemaValidation::Invalid(structural_errors));
    }

    // Re-parse from file for the typed representation. Keeping this in `parser`
    // avoids drift between parsing entry points.
    let schema = parse_schema_file(path)?;
    let semantic_errors = validate_semantic_schema(&schema);
    if !semantic_errors.is_empty() {
        return Ok(SchemaValidation::Invalid(semantic_errors));
    }

    Ok(SchemaValidation::Valid(schema))
}

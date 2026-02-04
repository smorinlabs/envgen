use anyhow::{Context, Result};
use jsonschema::{Draft, JSONSchema};
use serde_json::Value;
use std::sync::OnceLock;

static ROOT_SCHEMA: OnceLock<Value> = OnceLock::new();
static COMPILED_SCHEMA: OnceLock<JSONSchema> = OnceLock::new();

fn root_schema() -> Result<&'static Value> {
    if let Some(schema) = ROOT_SCHEMA.get() {
        return Ok(schema);
    }

    let parsed: Value =
        serde_json::from_str(super::JSON_SCHEMA).context("Embedded JSON Schema is invalid JSON")?;
    let _ = ROOT_SCHEMA.set(parsed);

    Ok(ROOT_SCHEMA.get().expect("ROOT_SCHEMA must be initialized"))
}

fn compiled_schema() -> Result<&'static JSONSchema> {
    if let Some(schema) = COMPILED_SCHEMA.get() {
        return Ok(schema);
    }

    let parsed = root_schema()?;
    let compiled = JSONSchema::options()
        .with_draft(Draft::Draft202012)
        .compile(parsed)
        .context("Embedded JSON Schema failed to compile")?;

    let _ = COMPILED_SCHEMA.set(compiled);
    Ok(COMPILED_SCHEMA
        .get()
        .expect("COMPILED_SCHEMA must be initialized"))
}

pub fn validate_instance(instance: &Value) -> Result<Vec<String>> {
    let schema = compiled_schema()?;
    let mut errors = Vec::new();

    if let Err(iter) = schema.validate(instance) {
        for error in iter {
            let path = error.instance_path.to_string();
            let message = error.to_string();
            if path.is_empty() {
                errors.push(message);
            } else {
                errors.push(format!("{path}: {message}"));
            }
        }
    }

    Ok(errors)
}

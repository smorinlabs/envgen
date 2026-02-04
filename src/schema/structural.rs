use anyhow::{bail, Context, Result};
use regex::Regex;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

static ROOT_SCHEMA: OnceLock<Value> = OnceLock::new();

#[derive(Clone, Default)]
struct JsonPointer(String);

impl JsonPointer {
    fn root() -> Self {
        Self(String::new())
    }

    fn push_prop(&self, prop: &str) -> Self {
        let mut next = self.0.clone();
        next.push('/');
        next.push_str(&escape_pointer_segment(prop));
        Self(next)
    }

    fn push_index(&self, idx: usize) -> Self {
        let mut next = self.0.clone();
        next.push('/');
        next.push_str(&idx.to_string());
        Self(next)
    }

    fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Clone)]
struct StructuralError {
    instance_path: JsonPointer,
    message: String,
}

impl StructuralError {
    fn new(instance_path: JsonPointer, message: impl Into<String>) -> Self {
        Self {
            instance_path,
            message: message.into(),
        }
    }
}

fn escape_pointer_segment(seg: &str) -> String {
    seg.replace('~', "~0").replace('/', "~1")
}

impl std::fmt::Display for StructuralError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.instance_path.as_str().is_empty() {
            write!(f, "{}", self.message)
        } else {
            write!(f, "{}: {}", self.instance_path.as_str(), self.message)
        }
    }
}

fn root_schema() -> Result<&'static Value> {
    if let Some(schema) = ROOT_SCHEMA.get() {
        return Ok(schema);
    }

    let parsed: Value =
        serde_json::from_str(super::JSON_SCHEMA).context("Embedded JSON Schema is invalid JSON")?;
    let _ = ROOT_SCHEMA.set(parsed);

    Ok(ROOT_SCHEMA.get().expect("ROOT_SCHEMA must be initialized"))
}

pub fn validate_instance(instance: &Value) -> Result<Vec<String>> {
    let schema = root_schema()?;
    let mut errors = Vec::new();
    validate_schema(schema, instance, schema, &JsonPointer::root(), &mut errors)?;
    Ok(errors.into_iter().map(|e| e.to_string()).collect())
}

fn validate_schema(
    schema: &Value,
    instance: &Value,
    root: &Value,
    instance_path: &JsonPointer,
    errors: &mut Vec<StructuralError>,
) -> Result<()> {
    // Boolean schemas (Draft 2020-12)
    if let Value::Bool(b) = schema {
        if *b {
            return Ok(());
        }
        errors.push(StructuralError::new(
            instance_path.clone(),
            "does not match schema (false)",
        ));
        return Ok(());
    }

    let schema_obj = match schema.as_object() {
        Some(o) => o,
        None => return Ok(()),
    };

    // $ref (local refs only)
    if let Some(Value::String(r)) = schema_obj.get("$ref") {
        let target = resolve_ref(root, r)
            .with_context(|| format!("Unsupported or unresolved $ref: \"{}\"", r))?;
        validate_schema(target, instance, root, instance_path, errors)?;
    }

    // allOf / anyOf / oneOf / not
    if let Some(Value::Array(all_of)) = schema_obj.get("allOf") {
        for sub in all_of {
            validate_schema(sub, instance, root, instance_path, errors)?;
        }
    }

    if let Some(Value::Array(any_of)) = schema_obj.get("anyOf") {
        let mut any_pass = false;
        for sub in any_of {
            let mut sub_errors = Vec::new();
            validate_schema(sub, instance, root, instance_path, &mut sub_errors)?;
            if sub_errors.is_empty() {
                any_pass = true;
                break;
            }
        }
        if !any_pass {
            errors.push(StructuralError::new(
                instance_path.clone(),
                "must match at least one schema in anyOf",
            ));
        }
    }

    if let Some(Value::Array(one_of)) = schema_obj.get("oneOf") {
        let mut pass_count = 0usize;
        for sub in one_of {
            let mut sub_errors = Vec::new();
            validate_schema(sub, instance, root, instance_path, &mut sub_errors)?;
            if sub_errors.is_empty() {
                pass_count += 1;
            }
        }
        if pass_count != 1 {
            errors.push(StructuralError::new(
                instance_path.clone(),
                "must match exactly one schema in oneOf",
            ));
        }
    }

    if let Some(not_schema) = schema_obj.get("not") {
        let mut sub_errors = Vec::new();
        validate_schema(not_schema, instance, root, instance_path, &mut sub_errors)?;
        if sub_errors.is_empty() {
            errors.push(StructuralError::new(
                instance_path.clone(),
                "must not match schema in not",
            ));
        }
    }

    // if / then / else
    if let Some(if_schema) = schema_obj.get("if") {
        let mut if_errors = Vec::new();
        validate_schema(if_schema, instance, root, instance_path, &mut if_errors)?;
        let if_passes = if_errors.is_empty();

        if if_passes {
            if let Some(then_schema) = schema_obj.get("then") {
                validate_schema(then_schema, instance, root, instance_path, errors)?;
            }
        } else if let Some(else_schema) = schema_obj.get("else") {
            validate_schema(else_schema, instance, root, instance_path, errors)?;
        }
    }

    // type check (if present). If it fails, stop evaluating other type-specific keywords.
    if let Some(expected_types) = schema_obj.get("type") {
        if !type_matches(expected_types, instance) {
            errors.push(StructuralError::new(
                instance_path.clone(),
                format!(
                    "expected type {}, got {}",
                    schema_type_display(expected_types),
                    instance_type_display(instance)
                ),
            ));
            return Ok(());
        }
    }

    // const / enum
    if let Some(const_value) = schema_obj.get("const") {
        if instance != const_value {
            errors.push(StructuralError::new(
                instance_path.clone(),
                format!("must be equal to {}", const_value),
            ));
        }
    }

    if let Some(Value::Array(enum_values)) = schema_obj.get("enum") {
        if !enum_values.iter().any(|v| v == instance) {
            errors.push(StructuralError::new(
                instance_path.clone(),
                "must be one of the allowed values",
            ));
        }
    }

    // string keywords
    if let Some(s) = instance.as_str() {
        if let Some(min_len) = schema_obj.get("minLength").and_then(|v| v.as_u64()) {
            if (s.chars().count() as u64) < min_len {
                errors.push(StructuralError::new(
                    instance_path.clone(),
                    format!("string must be at least {} characters", min_len),
                ));
            }
        }

        if let Some(Value::String(pattern)) = schema_obj.get("pattern") {
            let re = Regex::new(pattern)
                .with_context(|| format!("Invalid regex pattern in schema: {}", pattern))?;
            if !re.is_match(s) {
                errors.push(StructuralError::new(
                    instance_path.clone(),
                    format!("string does not match pattern {}", pattern),
                ));
            }
        }
    }

    // object keywords
    if let Some(obj) = instance.as_object() {
        if let Some(min_props) = schema_obj.get("minProperties").and_then(|v| v.as_u64()) {
            if (obj.len() as u64) < min_props {
                errors.push(StructuralError::new(
                    instance_path.clone(),
                    format!("object must have at least {} properties", min_props),
                ));
            }
        }

        if let Some(Value::Array(required)) = schema_obj.get("required") {
            for prop in required.iter().filter_map(|v| v.as_str()) {
                if !obj.contains_key(prop) {
                    errors.push(StructuralError::new(
                        instance_path.clone(),
                        format!("missing required property \"{}\"", prop),
                    ));
                }
            }
        }

        // propertyNames
        if let Some(property_names_schema) = schema_obj.get("propertyNames") {
            for key in sorted_object_keys(obj) {
                let key_value = Value::String(key.clone());
                let key_path = instance_path.push_prop(&key);
                validate_schema(property_names_schema, &key_value, root, &key_path, errors)?;
            }
        }

        // properties
        let properties = schema_obj
            .get("properties")
            .and_then(|v| v.as_object())
            .map(|m| m.iter().collect::<BTreeMap<_, _>>());

        if let Some(props) = &properties {
            for (prop, prop_schema) in props {
                if let Some(value) = obj.get(*prop) {
                    let next_path = instance_path.push_prop(prop);
                    validate_schema(prop_schema, value, root, &next_path, errors)?;
                }
            }
        }

        // additionalProperties
        if let Some(additional) = schema_obj.get("additionalProperties") {
            let known_props: BTreeSet<String> = properties
                .as_ref()
                .map(|m| m.keys().map(|k| (*k).to_string()).collect())
                .unwrap_or_default();

            match additional {
                Value::Bool(false) => {
                    for key in sorted_object_keys(obj) {
                        if !known_props.contains(&key) {
                            errors.push(StructuralError::new(
                                instance_path.push_prop(&key),
                                "unknown property",
                            ));
                        }
                    }
                }
                Value::Object(_) | Value::Bool(true) => {
                    for key in sorted_object_keys(obj) {
                        if known_props.contains(&key) {
                            continue;
                        }
                        let next_path = instance_path.push_prop(&key);
                        validate_schema(
                            additional,
                            obj.get(&key).unwrap(),
                            root,
                            &next_path,
                            errors,
                        )?;
                    }
                }
                _ => {
                    bail!("Unsupported additionalProperties value in embedded schema");
                }
            }
        }
    }

    // array keywords
    if let Some(arr) = instance.as_array() {
        if let Some(min_items) = schema_obj.get("minItems").and_then(|v| v.as_u64()) {
            if (arr.len() as u64) < min_items {
                errors.push(StructuralError::new(
                    instance_path.clone(),
                    format!("array must have at least {} items", min_items),
                ));
            }
        }

        if schema_obj
            .get("uniqueItems")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            for i in 0..arr.len() {
                for j in (i + 1)..arr.len() {
                    if arr[i] == arr[j] {
                        errors.push(StructuralError::new(
                            instance_path.clone(),
                            "array items must be unique",
                        ));
                        break;
                    }
                }
            }
        }

        if let Some(items_schema) = schema_obj.get("items") {
            for (idx, item) in arr.iter().enumerate() {
                let next_path = instance_path.push_index(idx);
                validate_schema(items_schema, item, root, &next_path, errors)?;
            }
        }
    }

    Ok(())
}

fn resolve_ref<'a>(root: &'a Value, reference: &str) -> Result<&'a Value> {
    if !reference.starts_with('#') {
        bail!("Only local refs are supported: {}", reference);
    }

    let pointer = &reference[1..];
    if pointer.is_empty() {
        return Ok(root);
    }
    if !pointer.starts_with('/') {
        bail!("Invalid local ref: {}", reference);
    }

    root.pointer(pointer)
        .ok_or_else(|| anyhow::anyhow!("Unresolved $ref: {}", reference))
}

fn type_matches(expected: &Value, instance: &Value) -> bool {
    match expected {
        Value::String(t) => instance_type_matches(t, instance),
        Value::Array(types) => types
            .iter()
            .filter_map(|v| v.as_str())
            .any(|t| instance_type_matches(t, instance)),
        _ => true,
    }
}

fn instance_type_matches(t: &str, instance: &Value) -> bool {
    match t {
        "object" => instance.is_object(),
        "array" => instance.is_array(),
        "string" => instance.is_string(),
        "boolean" => instance.is_boolean(),
        "number" => instance.is_number(),
        "integer" => instance.as_i64().is_some() || instance.as_u64().is_some(),
        "null" => instance.is_null(),
        _ => true,
    }
}

fn schema_type_display(expected: &Value) -> String {
    match expected {
        Value::String(t) => format!("\"{}\"", t),
        Value::Array(types) => {
            let mut parts: Vec<String> = types
                .iter()
                .filter_map(|v| v.as_str())
                .map(|t| format!("\"{}\"", t))
                .collect();
            parts.sort();
            format!("[{}]", parts.join(", "))
        }
        _ => "<unknown>".to_string(),
    }
}

fn instance_type_display(instance: &Value) -> &'static str {
    match instance {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(n) => {
            if n.is_i64() || n.is_u64() {
                "integer"
            } else {
                "number"
            }
        }
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn sorted_object_keys(obj: &serde_json::Map<String, Value>) -> Vec<String> {
    let mut keys: Vec<String> = obj.keys().cloned().collect();
    keys.sort();
    keys
}

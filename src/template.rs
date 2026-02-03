use anyhow::{bail, Result};
use regex::Regex;
use std::collections::HashMap;

/// Extract all placeholder names from a template string.
/// Placeholders are in the form `{name}`.
pub fn extract_placeholders(template: &str) -> Vec<String> {
    let re = Regex::new(r"\{([a-zA-Z_][a-zA-Z0-9_]*)\}").unwrap();
    re.captures_iter(template)
        .map(|cap| cap[1].to_string())
        .collect()
}

/// Expand all `{placeholder}` references in a template string using the provided context.
/// Returns an error if any placeholder cannot be resolved.
pub fn expand_template(template: &str, context: &HashMap<String, String>) -> Result<String> {
    let re = Regex::new(r"\{([a-zA-Z_][a-zA-Z0-9_]*)\}").unwrap();

    // First check all placeholders can be resolved
    let mut unresolved = Vec::new();
    for cap in re.captures_iter(template) {
        let name = &cap[1];
        if !context.contains_key(name) {
            unresolved.push(name.to_string());
        }
    }

    if !unresolved.is_empty() {
        bail!(
            "Unresolved template placeholders: {}",
            unresolved.join(", ")
        );
    }

    let result = re.replace_all(template, |caps: &regex::Captures| {
        let name = &caps[1];
        context.get(name).cloned().unwrap_or_default()
    });

    Ok(result.to_string())
}

/// Build a template context from environment config, variable key, and environment name.
pub fn build_context(
    env_name: &str,
    env_config: &std::collections::BTreeMap<String, String>,
    key: &str,
) -> HashMap<String, String> {
    let mut ctx = HashMap::new();

    // Add all environment config values
    for (k, v) in env_config {
        ctx.insert(k.clone(), v.clone());
    }

    // Add built-in values
    ctx.insert("environment".to_string(), env_name.to_string());
    ctx.insert("key".to_string(), key.to_string());

    ctx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_placeholders() {
        let template = "firebase functions:secrets:access {key} --project {firebase_project}";
        let placeholders = extract_placeholders(template);
        assert_eq!(placeholders, vec!["key", "firebase_project"]);
    }

    #[test]
    fn test_extract_no_placeholders() {
        let placeholders = extract_placeholders("no placeholders here");
        assert!(placeholders.is_empty());
    }

    #[test]
    fn test_expand_template() {
        let mut ctx = HashMap::new();
        ctx.insert("key".to_string(), "MY_SECRET".to_string());
        ctx.insert("project".to_string(), "my-project".to_string());

        let result = expand_template("echo {key} --project {project}", &ctx).unwrap();
        assert_eq!(result, "echo MY_SECRET --project my-project");
    }

    #[test]
    fn test_expand_template_unresolved() {
        let ctx = HashMap::new();
        let result = expand_template("echo {missing}", &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_build_context() {
        let mut env_config = std::collections::BTreeMap::new();
        env_config.insert("firebase_project".to_string(), "my-proj".to_string());

        let ctx = build_context("staging", &env_config, "MY_KEY");
        assert_eq!(ctx.get("environment").unwrap(), "staging");
        assert_eq!(ctx.get("key").unwrap(), "MY_KEY");
        assert_eq!(ctx.get("firebase_project").unwrap(), "my-proj");
    }
}

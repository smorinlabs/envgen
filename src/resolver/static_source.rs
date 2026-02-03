use anyhow::{bail, Result};

use crate::template;

/// Resolve a static variable value for the given environment.
pub fn resolve_static(
    var_name: &str,
    values: &std::collections::BTreeMap<String, String>,
    env_name: &str,
    env_config: &std::collections::BTreeMap<String, String>,
) -> Result<String> {
    let raw_value = match values.get(env_name) {
        Some(v) => v,
        None => bail!(
            "{}: no static value defined for environment \"{}\"",
            var_name,
            env_name
        ),
    };

    // Expand any template placeholders in the static value
    let ctx = template::build_context(env_name, env_config, var_name);
    template::expand_template(raw_value, &ctx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn test_resolve_static_simple() {
        let mut values = BTreeMap::new();
        values.insert("local".to_string(), "hello".to_string());
        let env_config = BTreeMap::new();

        let result = resolve_static("MY_VAR", &values, "local", &env_config).unwrap();
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_resolve_static_with_template() {
        let mut values = BTreeMap::new();
        values.insert("local".to_string(), "{project}-db".to_string());
        let mut env_config = BTreeMap::new();
        env_config.insert("project".to_string(), "myapp".to_string());

        let result = resolve_static("DB_NAME", &values, "local", &env_config).unwrap();
        assert_eq!(result, "myapp-db");
    }

    #[test]
    fn test_resolve_static_missing_env() {
        let values = BTreeMap::new();
        let env_config = BTreeMap::new();
        let result = resolve_static("MY_VAR", &values, "production", &env_config);
        assert!(result.is_err());
    }
}

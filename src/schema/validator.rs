use super::types::Schema;
use crate::template;
use std::collections::{BTreeSet, HashMap};

fn unresolved_template_placeholders(
    template_str: &str,
    env_config: &std::collections::BTreeMap<String, String>,
) -> Vec<String> {
    let mut missing: BTreeSet<String> = BTreeSet::new();
    for ph in template::extract_placeholders(template_str) {
        if ph == "key" || ph == "environment" {
            continue;
        }
        if !env_config.contains_key(&ph) {
            missing.insert(ph);
        }
    }
    missing.into_iter().collect()
}

fn format_unresolved_template_error(
    yaml_path: &str,
    value_kind: &str,
    env_name: &str,
    missing: &[String],
) -> Option<String> {
    if missing.is_empty() {
        return None;
    }

    let placeholders = missing
        .iter()
        .map(|k| format!("\"{{{}}}\"", k))
        .collect::<Vec<String>>()
        .join(", ");
    let fix_paths = missing
        .iter()
        .map(|k| format!("environments.{}.{}", env_name, k))
        .collect::<Vec<String>>()
        .join(", ");

    Some(format!(
        "{}: {} contains unresolved template placeholder{}: {}. Fix: add {} (or remove the placeholder{}).",
        yaml_path,
        value_kind,
        if missing.len() == 1 { "" } else { "s" },
        placeholders,
        fix_paths,
        if missing.len() == 1 { "" } else { "s" },
    ))
}

/// Validate a schema and return a list of errors. Empty list means valid.
pub fn validate_schema(schema: &Schema) -> Vec<String> {
    let mut errors = Vec::new();

    // Check schema_version
    if schema.schema_version != "2" {
        errors.push(format!(
            "Unsupported schema_version: \"{}\". Expected \"2\".",
            schema.schema_version
        ));
        return errors;
    }

    // Check metadata.destination has at least one entry
    if schema.metadata.destination.is_empty() {
        errors.push("metadata.destination must have at least one environment entry.".to_string());
    }

    // Check that destination environments are defined
    for env in schema.metadata.destination.keys() {
        if !schema.environments.contains_key(env) {
            errors.push(format!(
                "metadata.destination references environment \"{}\" which is not defined in environments.",
                env
            ));
        }
    }

    let env_names: Vec<&String> = schema.environments.keys().collect();

    // Validate each variable
    for (var_name, var) in &schema.variables {
        // Check description is not empty
        if var.description.trim().is_empty() {
            errors.push(format!("{}: description must not be empty.", var_name));
        }

        // Check environments references
        if let Some(var_envs) = &var.environments {
            for env in var_envs {
                if !schema.environments.contains_key(env) {
                    errors.push(format!(
                        "{}: references environment \"{}\" which is not defined in environments.",
                        var_name, env
                    ));
                }
            }
        }

        let applicable_envs: Vec<&String> = match &var.environments {
            Some(envs) => envs.iter().collect(),
            None => env_names.to_vec(),
        };

        let has_resolvers = var.resolvers.as_ref().is_some_and(|r| !r.is_empty());

        if has_resolvers {
            if var.source.is_some() {
                errors.push(format!(
                    "{}: cannot set both \"source\" and \"resolvers\". Choose one.",
                    var_name
                ));
            }
            if var.values.is_some() {
                errors.push(format!(
                    "{}: cannot set variable-level \"values\" when using \"resolvers\".",
                    var_name
                ));
            }

            let mut env_to_resolver: HashMap<String, usize> = HashMap::new();
            let resolvers = var.resolvers.as_ref().unwrap();

            for (idx, resolver) in resolvers.iter().enumerate() {
                // Check resolver environments references + overlaps
                if resolver.environments.is_empty() {
                    errors.push(format!(
                        "{}: resolver #{} must specify at least one environment.",
                        var_name,
                        idx + 1
                    ));
                }

                for env in &resolver.environments {
                    if !schema.environments.contains_key(env) {
                        errors.push(format!(
                            "{}: resolver references environment \"{}\" which is not defined in environments.",
                            var_name, env
                        ));
                    }
                    if !applicable_envs.contains(&env) {
                        errors.push(format!(
                            "{}: resolver references environment \"{}\" which is not applicable to this variable.",
                            var_name, env
                        ));
                    }
                    if env_to_resolver.contains_key(env) {
                        errors.push(format!(
                            "{}: resolver environments overlap for environment \"{}\".",
                            var_name, env
                        ));
                    } else {
                        env_to_resolver.insert(env.clone(), idx);
                    }
                }

                // Check resolver source is valid
                let source = resolver.source.as_str();
                if source != "static" && source != "manual" && !schema.sources.contains_key(source)
                {
                    errors.push(format!(
                        "{}: resolver source \"{}\" is not defined in sources.",
                        var_name, source
                    ));
                }

                // Check static resolvers have values for each resolver environment
                if source == "static" {
                    match &resolver.values {
                        None => errors.push(format!(
                            "{}: resolver source is \"static\" but no values map provided.",
                            var_name
                        )),
                        Some(values) => {
                            for env in &resolver.environments {
                                if !values.contains_key(env) {
                                    errors.push(format!(
                                        "{}: resolver source is \"static\" but no value provided for environment \"{}\".",
                                        var_name, env
                                    ));
                                    continue;
                                }

                                if let Some(env_config) = schema.environments.get(env) {
                                    let value = values.get(env).unwrap();
                                    let missing =
                                        unresolved_template_placeholders(value, env_config);
                                    let yaml_path = format!(
                                        "variables.{}.resolvers[{}].values.{}",
                                        var_name,
                                        idx + 1,
                                        env
                                    );
                                    if let Some(msg) = format_unresolved_template_error(
                                        &yaml_path,
                                        "static resolver value",
                                        env,
                                        &missing,
                                    ) {
                                        errors.push(msg);
                                    }
                                }
                            }
                        }
                    }
                }

                // Check source command template placeholders can be resolved (resolver-level)
                if source != "static" && source != "manual" {
                    if let Some(src) = schema.sources.get(source) {
                        for env_name in &resolver.environments {
                            if let Some(env_config) = schema.environments.get(env_name) {
                                let mut available_keys: Vec<String> =
                                    env_config.keys().cloned().collect();
                                available_keys.push("key".to_string());
                                available_keys.push("environment".to_string());

                                let placeholders = template::extract_placeholders(&src.command);
                                for ph in placeholders {
                                    if !available_keys.contains(&ph) {
                                        errors.push(format!(
                                            "{}: source command template references placeholder \"{{{}}}\" which cannot be resolved for environment \"{}\".",
                                            var_name, ph, env_name
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Ensure all applicable environments are covered by exactly one resolver
            for env in applicable_envs {
                if !env_to_resolver.contains_key(env) {
                    errors.push(format!(
                        "{}: no resolver provided for environment \"{}\".",
                        var_name, env
                    ));
                }
            }
        } else {
            // Single-source variables: require a source.
            let source = match var.source.as_deref() {
                Some(s) => s,
                None => {
                    errors.push(format!("{}: missing required field \"source\".", var_name));
                    continue;
                }
            };

            // Check source is valid
            if source != "static" && source != "manual" && !schema.sources.contains_key(source) {
                errors.push(format!(
                    "{}: source \"{}\" is not defined in sources.",
                    var_name, source
                ));
            }

            // Check static source has values for all applicable environments
            if source == "static" {
                match &var.values {
                    None => {
                        errors.push(format!(
                            "{}: source is \"static\" but no values map provided.",
                            var_name
                        ));
                    }
                    Some(values) => {
                        for env in &applicable_envs {
                            if !values.contains_key(*env) {
                                errors.push(format!(
                                    "{}: source is \"static\" but no value provided for environment \"{}\".",
                                    var_name, env
                                ));
                                continue;
                            }

                            if let Some(env_config) = schema.environments.get(*env) {
                                let value = values.get(*env).unwrap();
                                let missing = unresolved_template_placeholders(value, env_config);
                                let yaml_path =
                                    format!("variables.{}.values.{}", var_name, env);
                                if let Some(msg) = format_unresolved_template_error(
                                    &yaml_path,
                                    "static value",
                                    env,
                                    &missing,
                                ) {
                                    errors.push(msg);
                                }
                            }
                        }
                    }
                }
            }

            // Check source command template placeholders can be resolved
            if source != "static" && source != "manual" {
                if let Some(src) = schema.sources.get(source) {
                    for env_name in &applicable_envs {
                        if let Some(env_config) = schema.environments.get(*env_name) {
                            let mut available_keys: Vec<String> =
                                env_config.keys().cloned().collect();
                            available_keys.push("key".to_string());
                            available_keys.push("environment".to_string());

                            let placeholders = template::extract_placeholders(&src.command);
                            for ph in placeholders {
                                if !available_keys.contains(&ph) {
                                    errors.push(format!(
                                        "{}: source command template references placeholder \"{{{}}}\" which cannot be resolved for environment \"{}\".",
                                        var_name, ph, env_name
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Check that built-in source names are not redefined
    if schema.sources.contains_key("static") {
        errors.push("Source name \"static\" is built-in and must not be redefined.".to_string());
    }
    if schema.sources.contains_key("manual") {
        errors.push("Source name \"manual\" is built-in and must not be redefined.".to_string());
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::parser::parse_schema;

    fn errors_for(yaml: &str) -> Vec<String> {
        let schema = parse_schema(yaml).unwrap();
        validate_schema(&schema)
    }

    #[test]
    fn test_valid_schema() {
        let yaml = r#"
schema_version: "2"
metadata:
  description: "Test"
  destination:
    local: ".env"
environments:
  local:
    project: "test"
sources:
  my-source:
    command: "echo {key} --project {project}"
variables:
  FOO:
    description: "A variable"
    sensitive: false
    source: static
    values:
      local: "bar"
  BAZ:
    description: "Another variable"
    source: my-source
"#;
        let schema = parse_schema(yaml).unwrap();
        let errors = validate_schema(&schema);
        assert!(errors.is_empty(), "Expected no errors, got: {:?}", errors);
    }

    #[test]
    fn test_v2_resolver_environments_overlap() {
        let yaml = r#"
schema_version: "2"
metadata:
  description: "Test"
  destination:
    local: ".env"
    staging: ".env.staging"
environments:
  local: {}
  staging: {}
sources: {}
variables:
  FOO:
    description: "A variable"
    sensitive: false
    resolvers:
      - environments: [local, staging]
        source: static
        values:
          local: "a"
          staging: "b"
      - environments: [staging]
        source: static
        values:
          staging: "c"
"#;
        let errors = errors_for(yaml);
        assert!(
            errors
                .iter()
                .any(|e| e.contains("resolver environments overlap")),
            "Expected overlap error, got: {:?}",
            errors
        );
        assert!(
            errors
                .iter()
                .any(|e| e.contains("FOO") && e.contains("staging")),
            "Expected overlap to mention FOO/staging, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_v2_missing_resolver_coverage_for_environment() {
        let yaml = r#"
schema_version: "2"
metadata:
  description: "Test"
  destination:
    local: ".env"
    production: ".env.production"
environments:
  local: {}
  production: {}
sources: {}
variables:
  FOO:
    description: "A variable"
    resolvers:
      - environments: [local]
        source: static
        values:
          local: "a"
"#;
        let errors = errors_for(yaml);
        assert!(
            errors
                .iter()
                .any(|e| e.contains("no resolver provided") && e.contains("production")),
            "Expected missing resolver coverage for production, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_v2_cannot_set_source_and_resolvers() {
        let yaml = r#"
schema_version: "2"
metadata:
  description: "Test"
  destination:
    local: ".env"
environments:
  local: {}
sources: {}
variables:
  FOO:
    description: "A variable"
    source: static
    resolvers:
      - environments: [local]
        source: static
        values:
          local: "a"
"#;
        let errors = errors_for(yaml);
        assert!(
            errors.iter().any(|e| e.contains("cannot set both")
                && e.contains("\"source\"")
                && e.contains("\"resolvers\"")),
            "Expected source+resolvers conflict error, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_v2_static_resolver_requires_values() {
        let yaml = r#"
schema_version: "2"
metadata:
  description: "Test"
  destination:
    local: ".env"
environments:
  local: {}
sources: {}
variables:
  FOO:
    description: "A variable"
    resolvers:
      - environments: [local]
        source: static
"#;
        let errors = errors_for(yaml);
        assert!(
            errors
                .iter()
                .any(|e| e.contains("resolver source is \"static\"") && e.contains("values map")),
            "Expected missing values map error for static resolver, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_v2_resolver_command_template_unresolved_placeholders() {
        let yaml = r#"
schema_version: "2"
metadata:
  description: "Test"
  destination:
    local: ".env"
environments:
  local:
    project: "test"
sources:
  my-source:
    command: "echo {missing}"
variables:
  FOO:
    description: "A variable"
    resolvers:
      - environments: [local]
        source: my-source
"#;
        let errors = errors_for(yaml);
        assert!(
            errors
                .iter()
                .any(|e| e.contains("{missing}") && e.contains("local")),
            "Expected unresolved placeholder error for {{missing}}/local, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_static_value_unresolved_template_placeholders() {
        let yaml = r#"
schema_version: "2"
metadata:
  description: "Test"
  destination:
    local: ".env"
environments:
  local:
    project: "test"
sources: {}
variables:
  FOO:
    description: "A variable"
    source: static
    values:
      local: "{missing_key}"
"#;
        let errors = errors_for(yaml);
        assert!(
            errors.iter().any(|e| {
                e.contains("variables.FOO.values.local")
                    && e.contains("\"{missing_key}\"")
                    && e.contains("environments.local.missing_key")
            }),
            "Expected unresolved placeholder error for static value, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_v2_static_resolver_value_unresolved_template_placeholders() {
        let yaml = r#"
schema_version: "2"
metadata:
  description: "Test"
  destination:
    local: ".env"
environments:
  local:
    project: "test"
sources: {}
variables:
  FOO:
    description: "A variable"
    resolvers:
      - environments: [local]
        source: static
        values:
          local: "{missing_key}"
"#;
        let errors = errors_for(yaml);
        assert!(
            errors.iter().any(|e| {
                e.contains("variables.FOO.resolvers[1].values.local") && e.contains("\"{missing_key}\"")
            }),
            "Expected unresolved placeholder error for static resolver value, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_undefined_source() {
        let yaml = r#"
schema_version: "2"
metadata:
  description: "Test"
  destination:
    local: ".env"
environments:
  local:
    project: "test"
sources: {}
variables:
  FOO:
    description: "A variable"
    source: nonexistent
"#;
        let schema = parse_schema(yaml).unwrap();
        let errors = validate_schema(&schema);
        assert!(errors.iter().any(|e| e.contains("nonexistent")));
    }

    #[test]
    fn test_static_without_values() {
        let yaml = r#"
schema_version: "2"
metadata:
  description: "Test"
  destination:
    local: ".env"
environments:
  local:
    project: "test"
sources: {}
variables:
  FOO:
    description: "A variable"
    source: static
"#;
        let schema = parse_schema(yaml).unwrap();
        let errors = validate_schema(&schema);
        assert!(errors.iter().any(|e| e.contains("no values map")));
    }

    #[test]
    fn test_undefined_environment_in_variable() {
        let yaml = r#"
schema_version: "2"
metadata:
  description: "Test"
  destination:
    local: ".env"
environments:
  local:
    project: "test"
sources: {}
variables:
  FOO:
    description: "A variable"
    source: static
    environments: [local, production]
    values:
      local: "bar"
      production: "baz"
"#;
        let schema = parse_schema(yaml).unwrap();
        let errors = validate_schema(&schema);
        assert!(errors.iter().any(|e| e.contains("production")));
    }
}

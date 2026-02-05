use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Top-level schema structure for an envgen YAML schema file.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Schema {
    pub schema_version: String,
    pub metadata: Metadata,
    pub environments: BTreeMap<String, BTreeMap<String, String>>,
    #[serde(default)]
    pub sources: BTreeMap<String, Source>,
    pub variables: BTreeMap<String, Variable>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Metadata {
    pub description: String,
    pub destination: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub command: String,

    #[serde(default)]
    pub label: Option<String>,

    #[serde(default)]
    pub url: Option<String>,

    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Variable {
    pub description: String,

    #[serde(default = "default_sensitive")]
    pub sensitive: bool,

    /// Key into `sources`, or `static` / `manual`.
    ///
    /// For schema v2, this may be omitted when `resolvers` is used.
    #[serde(default)]
    pub source: Option<String>,

    #[serde(default)]
    pub source_key: Option<String>,

    #[serde(default)]
    pub source_instructions: Option<String>,

    /// Which environments this variable applies to. If None, applies to all.
    #[serde(default)]
    pub environments: Option<Vec<String>>,

    /// Inline values per environment (required when source = "static").
    #[serde(default)]
    pub values: Option<BTreeMap<String, String>>,

    /// Schema v2: Per-environment resolver bindings for this variable.
    ///
    /// When present, the active resolver is selected by environment name.
    #[serde(default)]
    pub resolvers: Option<Vec<VariableResolver>>,

    #[serde(default = "default_required")]
    pub required: bool,

    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VariableResolver {
    pub environments: Vec<String>,
    pub source: String,

    #[serde(default)]
    pub label: Option<String>,

    #[serde(default)]
    pub url: Option<String>,

    #[serde(default)]
    pub description: Option<String>,

    #[serde(default)]
    pub source_key: Option<String>,

    /// Inline values per environment (required when source = "static").
    #[serde(default)]
    pub values: Option<BTreeMap<String, String>>,
}

fn default_sensitive() -> bool {
    true
}

fn default_required() -> bool {
    true
}

impl Variable {
    /// Returns true if this variable applies to the given environment.
    pub fn applies_to(&self, env: &str) -> bool {
        match &self.environments {
            Some(envs) => envs.iter().any(|e| e == env),
            None => true,
        }
    }

    /// Returns the resolver that applies to the given environment (schema v2).
    pub fn resolver_for_env(&self, env: &str) -> Option<&VariableResolver> {
        self.resolvers
            .as_ref()
            .and_then(|rs| rs.iter().find(|r| r.environments.iter().any(|e| e == env)))
    }

    /// Returns the effective source name for the given environment.
    pub fn effective_source_for_env(&self, env: &str) -> Option<&str> {
        self.resolver_for_env(env)
            .map(|r| r.source.as_str())
            .or(self.source.as_deref())
    }

    /// Returns the key to use in source command templates.
    pub fn effective_key_for_env(&self, var_name: &str, env: &str) -> String {
        self.resolver_for_env(env)
            .and_then(|r| r.source_key.clone())
            .or_else(|| self.source_key.clone())
            .unwrap_or_else(|| var_name.to_string())
    }

    /// Returns the values map to use for the given environment (static source only).
    pub fn values_for_env(&self, env: &str) -> Option<&BTreeMap<String, String>> {
        self.resolver_for_env(env)
            .and_then(|r| r.values.as_ref())
            .or(self.values.as_ref())
    }
}

impl Schema {
    /// Returns the list of environment names defined in the schema.
    pub fn environment_names(&self) -> Vec<String> {
        self.environments.keys().cloned().collect()
    }

    /// Returns the destination path for the given environment.
    pub fn destination_for(&self, env: &str) -> Option<&String> {
        self.metadata.destination.get(env)
    }
}

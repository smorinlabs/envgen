use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Top-level schema structure for an envtool YAML schema file.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Schema {
    pub schema_version: String,
    pub metadata: Metadata,
    pub environments: BTreeMap<String, BTreeMap<String, String>>,
    #[serde(default)]
    pub sources: BTreeMap<String, Source>,
    pub variables: BTreeMap<String, Variable>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Metadata {
    pub description: String,
    pub destination: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Source {
    pub command: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Variable {
    pub description: String,

    #[serde(default = "default_sensitive")]
    pub sensitive: bool,

    pub source: String,

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

    #[serde(default = "default_required")]
    pub required: bool,

    #[serde(default)]
    pub notes: Option<String>,
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

    /// Returns the key to use in source command templates.
    pub fn effective_key(&self, var_name: &str) -> String {
        self.source_key
            .clone()
            .unwrap_or_else(|| var_name.to_string())
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

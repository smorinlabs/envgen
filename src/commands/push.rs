use anyhow::{bail, Result};
use colored::Colorize;
use std::path::PathBuf;

use crate::schema::validation::{load_and_validate_schema_file, SchemaValidation};

pub struct PushOptions {
    pub schema_path: PathBuf,
    pub env_name: String,
    pub var_name: String,
    pub from_file: Option<PathBuf>,
    pub yes: bool,
    pub show_secret: bool,
    pub dry_run: bool,
    pub source_timeout: u64,
    pub allow_empty: bool,
}

/// Run the `push` command. Returns the process exit code.
pub async fn run_push(opts: PushOptions) -> Result<i32> {
    let schema = match load_and_validate_schema_file(&opts.schema_path)? {
        SchemaValidation::Valid(s) => s,
        SchemaValidation::Invalid(errors) => {
            println!("{} Schema errors:", "✗".red());
            for e in &errors {
                println!("  - {}", e);
            }
            bail!("Schema validation failed. Fix errors before pushing.");
        }
    };

    if !schema.environments.contains_key(&opts.env_name) {
        let available: Vec<String> = schema.environment_names();
        bail!(
            "Environment \"{}\" not found. Available: {}",
            opts.env_name,
            available.join(", ")
        );
    }

    let var = match schema.variables.get(&opts.var_name) {
        Some(v) => v,
        None => bail!("Variable '{}' not found in schema", opts.var_name),
    };

    if !var.applies_to(&opts.env_name) {
        let allowed = var
            .environments
            .as_ref()
            .map(|e| e.join(", "))
            .unwrap_or_else(|| "<all>".to_string());
        bail!(
            "Variable '{}' is not applicable to env '{}' (allowed: [{}])",
            opts.var_name,
            opts.env_name,
            allowed
        );
    }

    let source_name = match var.effective_source_for_env(&opts.env_name) {
        Some(s) => s.to_string(),
        None => bail!(
            "No source configured for variable '{}' in env '{}'",
            opts.var_name,
            opts.env_name
        ),
    };

    if source_name == "static" {
        bail!(
            "Cannot push '{}' for env={}: source is 'static'. Static values are defined inline — edit the variable's values: block.",
            opts.var_name,
            opts.env_name
        );
    }

    if source_name == "manual" {
        bail!(
            "Cannot push '{}' for env={}: source is 'manual'. Manual sources have no remote — store the value in your password manager.",
            opts.var_name,
            opts.env_name
        );
    }

    let source = match schema.sources.get(&source_name) {
        Some(s) => s,
        None => bail!(
            "Source '{}' is not defined in sources (referenced by variable '{}').",
            source_name,
            opts.var_name
        ),
    };

    if source.push_command.is_none() {
        bail!(
            "Cannot push '{}' for env={}. Source '{}' has no push_command defined.\n\nAdd to your schema:\n  sources:\n    {}:\n      push_command: \"<e.g. gcloud secrets versions add {{key}} --data-file=- --project={{app_slug}}>\"",
            opts.var_name,
            opts.env_name,
            source_name,
            source_name
        );
    }

    // Value input + execution lands in later tasks.
    bail!("push: value input not yet implemented");
}

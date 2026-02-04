use anyhow::{bail, Result};
use colored::Colorize;
use std::path::Path;

use crate::output;
use crate::schema::validation::{load_and_validate_schema_file, SchemaValidation};

/// Output format for the list command.
pub enum ListFormat {
    Table,
    Json,
}

impl ListFormat {
    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "table" => Ok(ListFormat::Table),
            "json" => Ok(ListFormat::Json),
            _ => anyhow::bail!("Unknown format: \"{}\". Expected \"table\" or \"json\".", s),
        }
    }
}

/// Run the `list` command: display variables defined in the schema.
pub fn run_list(schema_path: &Path, env_filter: Option<&str>, format: ListFormat) -> Result<()> {
    let schema = match load_and_validate_schema_file(schema_path)? {
        SchemaValidation::Valid(schema) => schema,
        SchemaValidation::Invalid(errors) => {
            println!("{} Schema errors:", "✗".red());
            for error in &errors {
                println!("  - {}", error);
            }
            bail!("Schema validation failed. Fix errors before listing.");
        }
    };

    // Validate env filter if provided
    if let Some(env) = env_filter {
        if !schema.environments.contains_key(env) {
            let available: Vec<String> = schema.environment_names();
            bail!(
                "Environment \"{}\" not found. Available: {}",
                env,
                available.join(", ")
            );
        }
    }

    match format {
        ListFormat::Table => {
            println!(
                "Schema: {} ({})\n",
                schema_path.display(),
                schema.metadata.description.trim()
            );
            let table_output = output::format_variable_table(&schema, env_filter);
            println!("{}", table_output);
        }
        ListFormat::Json => {
            let json = output::format_variable_json(&schema, env_filter)?;
            println!("{}", json);
        }
    }

    Ok(())
}

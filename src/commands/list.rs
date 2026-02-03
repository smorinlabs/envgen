use anyhow::Result;
use std::path::Path;

use crate::output;
use crate::schema::parser::parse_schema_file;

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
pub fn run_list(
    schema_path: &Path,
    env_filter: Option<&str>,
    format: ListFormat,
) -> Result<()> {
    let schema = parse_schema_file(schema_path)?;

    // Validate env filter if provided
    if let Some(env) = env_filter {
        if !schema.environments.contains_key(env) {
            let available: Vec<String> = schema.environment_names();
            anyhow::bail!(
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

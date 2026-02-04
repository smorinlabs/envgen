use anyhow::{bail, Result};
use colored::Colorize;
use std::path::Path;

use crate::output;
use crate::schema::validation::{load_and_validate_schema_file, SchemaValidation};

/// Run the `docs` command: generate Markdown documentation for a schema file.
pub fn run_docs(schema_path: &Path, env_filter: Option<&str>) -> Result<()> {
    let schema = match load_and_validate_schema_file(schema_path)? {
        SchemaValidation::Valid(schema) => schema,
        SchemaValidation::Invalid(errors) => {
            println!("{} Schema errors:", "✗".red());
            for error in &errors {
                println!("  - {}", error);
            }
            bail!("Schema validation failed. Fix errors before generating docs.");
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

    let markdown = output::format_schema_docs_markdown(schema_path, &schema, env_filter)?;
    print!("{}", markdown);
    Ok(())
}

use anyhow::Result;
use colored::Colorize;
use std::path::Path;

use crate::schema::parser::parse_schema_file;
use crate::schema::validator::validate_schema;

/// Run the `check` command: validate a schema file.
pub fn run_check(schema_path: &Path) -> Result<bool> {
    let schema = parse_schema_file(schema_path)?;
    let errors = validate_schema(&schema);

    if errors.is_empty() {
        let var_count = schema.variables.len();
        let env_count = schema.environments.len();
        let source_count = schema.sources.len();

        println!(
            "{} Schema valid: {} variable{}, {} environment{}, {} source{}",
            "✓".green(),
            var_count,
            if var_count == 1 { "" } else { "s" },
            env_count,
            if env_count == 1 { "" } else { "s" },
            source_count,
            if source_count == 1 { "" } else { "s" },
        );
        Ok(true)
    } else {
        println!("{} Schema errors:", "✗".red());
        for error in &errors {
            println!("  - {}", error);
        }
        Ok(false)
    }
}

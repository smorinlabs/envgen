use anyhow::{bail, Result};
use std::fs;
use std::path::PathBuf;

use crate::schema;

pub struct SchemaExportOptions {
    pub output_path: Option<PathBuf>,
    pub force: bool,
    pub quiet: bool,
}

fn resolve_output_path(output: Option<PathBuf>) -> PathBuf {
    match output {
        Some(path) => {
            if path.exists() && path.is_dir() {
                path.join(schema::JSON_SCHEMA_FILENAME)
            } else {
                path
            }
        }
        None => PathBuf::from(schema::JSON_SCHEMA_FILENAME),
    }
}

pub fn run_schema_print() -> Result<()> {
    print!("{}", schema::JSON_SCHEMA);
    Ok(())
}

pub fn run_schema_export(opts: SchemaExportOptions) -> Result<()> {
    let dest_path = resolve_output_path(opts.output_path);

    if dest_path.exists() && dest_path.is_dir() {
        bail!(
            "Destination path \"{}\" is a directory. Provide a file path.",
            dest_path.display()
        );
    }

    if dest_path.exists() && !opts.force {
        bail!(
            "Destination file \"{}\" already exists. Use --force to overwrite.",
            dest_path.display()
        );
    }

    fs::write(&dest_path, schema::JSON_SCHEMA)?;

    if !opts.quiet {
        println!("Wrote JSON Schema to {}", dest_path.display());
    }

    Ok(())
}

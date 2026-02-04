use anyhow::{bail, Result};
use std::fs;
use std::path::PathBuf;

const SAMPLE_SCHEMA: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/schemas/envgen.sample.yaml"
));

pub struct InitOptions {
    pub output_path: Option<PathBuf>,
    pub force: bool,
    pub quiet: bool,
}

fn resolve_output_path(output: Option<PathBuf>) -> PathBuf {
    match output {
        Some(path) => {
            if path.exists() && path.is_dir() {
                path.join("env.dev.yaml")
            } else {
                path
            }
        }
        None => PathBuf::from("env.dev.yaml"),
    }
}

pub fn run_init(opts: InitOptions) -> Result<()> {
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

    if let Some(parent) = dest_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&dest_path, SAMPLE_SCHEMA)?;

    if !opts.quiet {
        println!("Wrote sample schema to {}", dest_path.display());
    }

    Ok(())
}

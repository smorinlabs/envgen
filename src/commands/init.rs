use anyhow::{bail, Result};
use std::fs;
use std::path::PathBuf;

const SAMPLE_SCHEMA: &str = r#"# envgen schema (v2)
#
# Next steps:
# 1. Update metadata.destination to match where you want env files written.
# 2. Fill in environments.* values (used by {placeholders} in command sources).
# 3. Add sources and variables for your project.
# 4. Run: envgen pull --schema env.dev.yaml --env dev
#
# This file includes inline comments explaining common fields. Remove or edit them as you like.

schema_version: "2"

metadata:
  # Human-readable description. Used in list output and docs.
  description: "Sample envgen schema. Replace with your project description."
  # Destination .env paths per environment.
  destination:
    local: ".env.local"
    dev: ".env.dev"
    stg: ".env.stg"
    prod: ".env.prod"

environments:
  # Free-form per-environment config. These keys can be referenced by {placeholders}
  # in source commands (e.g., {app_slug}).
  local:
    app_slug: "envgen-demo"
  dev:
    app_slug: "envgen-demo"
  stg:
    app_slug: "envgen-demo"
  prod:
    app_slug: "envgen-demo"

sources:
  # Source names are identifiers you choose. "command_name" is not a reserved keyword.
  command_name:
    # Commands can reference {key}, {environment}, and environment config keys like {app_slug}.
    command: "bash -lc 'echo {app_slug}-{environment}-{key}-$(date +%Y%m%d%H%M%S)'"

variables:
  # Each variable needs a description and a source.
  #
  # Common optional fields:
  # - sensitive: true|false (defaults to true; controls masking in output)
  # - source_instructions: shown for manual sources to help people find the value
  # - notes: free-form metadata for humans
  # - environments: limit a variable to specific environments
  # - resolvers: (schema v2) pick different sources per environment

  APP_NAME:
    description: "App display name (example of a static variable)."
    sensitive: false
    source: static
    values:
      local: "Envgen Local"
      dev: "Envgen Dev"
      stg: "Envgen Stg"
      prod: "Envgen Prod"

  API_TOKEN:
    description: "API token used for local development."
    source: manual
    source_instructions: "Create a token in your admin UI and paste it here."
    notes: "Rotate quarterly."

  BUILD_ID:
    description: "Dummy build identifier generated via a command source."
    source: command_name
    environments: [local, dev, stg, prod]
"#;

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

    fs::write(&dest_path, SAMPLE_SCHEMA)?;

    if !opts.quiet {
        println!("Wrote sample schema to {}", dest_path.display());
    }

    Ok(())
}

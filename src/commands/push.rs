use anyhow::{bail, Context, Result};
use colored::Colorize;
use std::io::{IsTerminal, Read};
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

#[derive(Debug)]
enum InputMode {
    File(PathBuf),
    StdinPipe,
    Prompt,
}

fn decide_input_mode(from_file: Option<PathBuf>, stdin_is_tty: bool) -> Result<InputMode> {
    if let Some(path) = from_file {
        return Ok(InputMode::File(path));
    }
    if !stdin_is_tty {
        return Ok(InputMode::StdinPipe);
    }
    Ok(InputMode::Prompt)
}

fn strip_one_trailing_newline(s: &str) -> String {
    if let Some(stripped) = s.strip_suffix("\r\n") {
        return stripped.to_string();
    }
    if let Some(stripped) = s.strip_suffix('\n') {
        return stripped.to_string();
    }
    s.to_string()
}

fn read_value_from_file(path: &std::path::Path) -> Result<String> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read --from-file path: {}", path.display()))?;
    Ok(strip_one_trailing_newline(&raw))
}

fn read_value_from_stdin_pipe() -> Result<String> {
    let mut buf = String::new();
    std::io::stdin()
        .read_to_string(&mut buf)
        .context("Failed to read value from stdin")?;
    Ok(strip_one_trailing_newline(&buf))
}

fn prompt_for_value(var_name: &str) -> Result<String> {
    use dialoguer::Password;
    let prompt = format!("Enter value for {}", var_name);
    let value: String = Password::new()
        .with_prompt(prompt)
        .interact()
        .context("Failed to read value from interactive prompt")?;
    Ok(value)
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

    let stdin_is_tty = std::io::stdin().is_terminal();
    let mode = decide_input_mode(opts.from_file.clone(), stdin_is_tty)?;
    let value = match mode {
        InputMode::File(p) => read_value_from_file(&p)?,
        InputMode::StdinPipe => read_value_from_stdin_pipe()?,
        InputMode::Prompt => prompt_for_value(&opts.var_name)?,
    };

    if value.is_empty() && !opts.allow_empty {
        bail!(
            "Refusing to push empty value for '{}'. Pass --allow-empty to override.",
            opts.var_name
        );
    }

    if opts.dry_run {
        let displayed = if opts.show_secret {
            value.clone()
        } else {
            "********".to_string()
        };
        println!();
        println!("variable:    {}", opts.var_name);
        println!("environment: {}", opts.env_name);
        println!("source:      {}", source_name);
        println!("value:       {}", displayed);
        println!();
        println!("(dry-run: command resolution lands in a later task)");
        return Ok(0);
    }

    bail!("push: command resolution not yet implemented");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn decide_input_mode_prefers_from_file() {
        let mode =
            decide_input_mode(Some(PathBuf::from("/tmp/x")), /*stdin_is_tty=*/ false).unwrap();
        assert!(matches!(mode, InputMode::File(_)));
    }

    #[test]
    fn decide_input_mode_pipe_when_stdin_not_tty() {
        let mode = decide_input_mode(None, /*stdin_is_tty=*/ false).unwrap();
        assert!(matches!(mode, InputMode::StdinPipe));
    }

    #[test]
    fn decide_input_mode_prompt_when_stdin_tty() {
        let mode = decide_input_mode(None, /*stdin_is_tty=*/ true).unwrap();
        assert!(matches!(mode, InputMode::Prompt));
    }

    #[test]
    fn strip_one_trailing_newline_strips_lf() {
        assert_eq!(strip_one_trailing_newline("abc\n"), "abc");
    }

    #[test]
    fn strip_one_trailing_newline_strips_crlf() {
        assert_eq!(strip_one_trailing_newline("abc\r\n"), "abc");
    }

    #[test]
    fn strip_one_trailing_newline_strips_only_one() {
        assert_eq!(strip_one_trailing_newline("abc\n\n"), "abc\n");
    }

    #[test]
    fn strip_one_trailing_newline_passthrough() {
        assert_eq!(strip_one_trailing_newline("abc"), "abc");
    }
}

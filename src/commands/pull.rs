use anyhow::{bail, Result};
use colored::Colorize;
use std::path::PathBuf;

use crate::output;
use crate::resolver::{command_source, manual_source, static_source};
use crate::schema::parser::parse_schema_file;
use crate::schema::validator::validate_schema;

pub struct PullOptions {
    pub schema_path: PathBuf,
    pub env_name: String,
    pub dry_run: bool,
    pub unmask: bool,
    pub force: bool,
    pub non_interactive: bool,
    pub output_path: Option<PathBuf>,
    pub timeout: u64,
}

/// A resolved variable result.
enum ResolveResult {
    Success(String, String), // (var_name, value)
    Skipped(String, String), // (var_name, reason)
    Failed(String, String),  // (var_name, error)
}

/// Run the `pull` command: resolve variables and write the .env file.
pub async fn run_pull(opts: PullOptions) -> Result<bool> {
    // Parse and validate schema
    let schema = parse_schema_file(&opts.schema_path)?;
    let errors = validate_schema(&schema);
    if !errors.is_empty() {
        println!("{} Schema errors:", "✗".red());
        for error in &errors {
            println!("  - {}", error);
        }
        bail!("Schema validation failed. Fix errors before pulling.");
    }

    // Validate environment
    if !schema.environments.contains_key(&opts.env_name) {
        let available: Vec<String> = schema.environment_names();
        bail!(
            "Environment \"{}\" not found. Available: {}",
            opts.env_name,
            available.join(", ")
        );
    }

    let env_config = schema.environments.get(&opts.env_name).unwrap();

    // Determine destination path
    let dest_path = if let Some(ref output) = opts.output_path {
        output.clone()
    } else {
        match schema.destination_for(&opts.env_name) {
            Some(p) => PathBuf::from(p),
            None => bail!(
                "No destination defined for environment \"{}\" in metadata.destination.",
                opts.env_name
            ),
        }
    };

    // Dry run header
    if opts.dry_run {
        println!();
        println!("Schema:      {}", opts.schema_path.display());
        println!("Environment: {}", opts.env_name);
        let exists_str = if dest_path.exists() {
            "exists"
        } else {
            "does not exist"
        };
        println!("Destination: {} ({})", dest_path.display(), exists_str);
        println!();
        println!("Variables to resolve:");
        println!();

        let mut command_count = 0;
        let mut static_manual_count = 0;
        let mut var_count = 0;

        for (var_name, var) in &schema.variables {
            if !var.applies_to(&opts.env_name) {
                continue;
            }
            var_count += 1;

            let source = match var.effective_source_for_env(&opts.env_name) {
                Some(s) => s,
                None => {
                    println!("  {}", var_name);
                    println!("    source:  <missing>");
                    println!();
                    continue;
                }
            };
            if source == "static" {
                static_manual_count += 1;
                let value = match var.values_for_env(&opts.env_name) {
                    Some(values) => match static_source::resolve_static(
                        var_name,
                        values,
                        &opts.env_name,
                        env_config,
                    ) {
                        Ok(v) => {
                            if var.sensitive && !opts.unmask {
                                output::mask_value(&v, false)
                            } else {
                                v
                            }
                        }
                        Err(e) => format!("<error: {}>", e),
                    },
                    None => "<missing>".to_string(),
                };
                println!("  {}", var_name);
                println!("    source:  static");
                println!("    value:   {}", value);
                println!();
            } else if source == "manual" {
                static_manual_count += 1;
                println!("  {}", var_name);
                println!("    source:  manual (interactive prompt)");
                if let Some(instructions) = &var.source_instructions {
                    println!("    instructions: {}", instructions.trim());
                }
                println!();
            } else {
                command_count += 1;
                if let Some(src) = schema.sources.get(source) {
                    let key = var.effective_key_for_env(var_name, &opts.env_name);
                    let cmd = command_source::build_command(
                        &src.command,
                        var_name,
                        Some(&key),
                        &opts.env_name,
                        env_config,
                    )
                    .unwrap_or_else(|e| format!("<error: {}>", e));

                    println!("  {}", var_name);
                    println!("    source:  {}", source);
                    println!("    command: {}", cmd);
                    println!();
                }
            }
        }

        println!(
            "{} variable{} would be written to {}",
            var_count,
            if var_count == 1 { "" } else { "s" },
            dest_path.display()
        );
        println!(
            "{} command{} would be executed ({} static/manual)",
            command_count,
            if command_count == 1 { "" } else { "s" },
            static_manual_count
        );
        return Ok(true);
    }

    // Check if destination exists (non dry-run)
    if dest_path.exists() && !opts.force {
        bail!(
            "Destination file \"{}\" already exists. Use --force to overwrite.",
            dest_path.display()
        );
    }

    // Count applicable variables
    let applicable_vars: Vec<(&String, &crate::schema::types::Variable)> = schema
        .variables
        .iter()
        .filter(|(_, v)| v.applies_to(&opts.env_name))
        .collect();

    println!(
        "\nPulling {} variable{} for environment \"{}\"...\n",
        applicable_vars.len(),
        if applicable_vars.len() == 1 { "" } else { "s" },
        opts.env_name
    );

    // Collect commands to run in parallel
    let mut command_tasks: Vec<(String, String, String, bool)> = Vec::new(); // (var_name, source_name, command, required)
    let mut static_results: Vec<ResolveResult> = Vec::new();
    let mut manual_vars: Vec<(String, String, Option<String>, bool)> = Vec::new(); // (var_name, description, instructions, required)

    for (var_name, var) in &applicable_vars {
        let source = match var.effective_source_for_env(&opts.env_name) {
            Some(s) => s,
            None => {
                static_results.push(ResolveResult::Failed(
                    var_name.to_string(),
                    "No source configured for this variable/environment".to_string(),
                ));
                continue;
            }
        };
        if source == "static" {
            match var.values_for_env(&opts.env_name) {
                Some(values) => match static_source::resolve_static(
                    var_name,
                    values,
                    &opts.env_name,
                    env_config,
                ) {
                    Ok(value) => {
                        static_results.push(ResolveResult::Success(var_name.to_string(), value));
                    }
                    Err(e) => {
                        static_results
                            .push(ResolveResult::Failed(var_name.to_string(), e.to_string()));
                    }
                },
                None => {
                    static_results.push(ResolveResult::Failed(
                        var_name.to_string(),
                        "No values map provided".to_string(),
                    ));
                }
            }
        } else if source == "manual" {
            manual_vars.push((
                var_name.to_string(),
                var.description.clone(),
                var.source_instructions.clone(),
                var.required,
            ));
        } else if let Some(src) = schema.sources.get(source) {
            let key = var.effective_key_for_env(var_name, &opts.env_name);
            match command_source::build_command(
                &src.command,
                var_name,
                Some(&key),
                &opts.env_name,
                env_config,
            ) {
                Ok(cmd) => {
                    command_tasks.push((
                        var_name.to_string(),
                        source.to_string(),
                        cmd,
                        var.required,
                    ));
                }
                Err(e) => {
                    static_results.push(ResolveResult::Failed(var_name.to_string(), e.to_string()));
                }
            }
        }
    }

    // Execute all command tasks in parallel
    let mut handles = Vec::new();
    for (var_name, _source_name, cmd, required) in command_tasks {
        let timeout = opts.timeout;
        handles.push(tokio::spawn(async move {
            match command_source::execute_command(&cmd, timeout).await {
                Ok(result) => ResolveResult::Success(var_name, result.value),
                Err(e) => {
                    if required {
                        ResolveResult::Failed(var_name, e.to_string())
                    } else {
                        ResolveResult::Skipped(var_name, e.to_string())
                    }
                }
            }
        }));
    }

    let mut all_results: Vec<ResolveResult> = static_results;

    // Collect parallel results
    for handle in handles {
        let result = handle.await?;
        all_results.push(result);
    }

    // Handle manual prompts (must be sequential)
    for (var_name, description, instructions, required) in manual_vars {
        match manual_source::resolve_manual(
            &var_name,
            &description,
            instructions.as_deref(),
            &opts.env_name,
            env_config,
            opts.non_interactive,
        ) {
            Ok(Some(value)) => {
                all_results.push(ResolveResult::Success(var_name, value));
            }
            Ok(None) => {
                // Skipped in non-interactive mode
                all_results.push(ResolveResult::Skipped(
                    var_name,
                    "skipped in non-interactive mode".to_string(),
                ));
            }
            Err(e) => {
                if required {
                    all_results.push(ResolveResult::Failed(var_name, e.to_string()));
                } else {
                    all_results.push(ResolveResult::Skipped(var_name, e.to_string()));
                }
            }
        }
    }

    // Build ordered output, preserving schema variable order
    let mut resolved_vars: Vec<(String, String)> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut failed_required = 0;

    // We need to match results back to the original variable order
    let var_order: Vec<String> = applicable_vars
        .iter()
        .map(|(name, _)| name.to_string())
        .collect();

    // Build lookup map from results
    let mut result_map: std::collections::HashMap<String, ResolveResult> =
        std::collections::HashMap::new();
    for result in all_results {
        let name = match &result {
            ResolveResult::Success(n, _) => n.clone(),
            ResolveResult::Skipped(n, _) => n.clone(),
            ResolveResult::Failed(n, _) => n.clone(),
        };
        result_map.insert(name, result);
    }

    for var_name in &var_order {
        let var = schema.variables.get(var_name).unwrap();
        let source_display = var
            .effective_source_for_env(&opts.env_name)
            .unwrap_or("<missing>");

        if let Some(result) = result_map.remove(var_name) {
            match result {
                ResolveResult::Success(_, value) => {
                    println!("  {} {:<24} ({})", "✓".green(), var_name, source_display);
                    resolved_vars.push((var_name.clone(), value));
                }
                ResolveResult::Skipped(_, reason) => {
                    println!(
                        "  {} {:<24} ({}) — {}",
                        "⊘".yellow(),
                        var_name,
                        source_display,
                        reason
                    );
                    warnings.push(format!(
                        "{} could not be resolved (required={})",
                        var_name, var.required
                    ));
                }
                ResolveResult::Failed(_, error) => {
                    println!(
                        "  {} {:<24} ({}) — {}",
                        "✗".red(),
                        var_name,
                        source_display,
                        error
                    );
                    warnings.push(format!(
                        "{} could not be resolved (required={})",
                        var_name, var.required
                    ));
                    if var.required {
                        failed_required += 1;
                    }
                }
            }
        }
    }

    println!();

    // Write output file
    if !resolved_vars.is_empty() {
        output::write_env_file(
            &dest_path,
            &opts.schema_path.to_string_lossy(),
            &opts.env_name,
            &resolved_vars,
        )?;
        println!(
            "Wrote {} variable{} to {}",
            resolved_vars.len(),
            if resolved_vars.len() == 1 { "" } else { "s" },
            dest_path.display()
        );
    } else {
        println!("No variables resolved. Output file not written.");
    }

    if !warnings.is_empty() {
        println!(
            "{} warning{}: {}",
            warnings.len(),
            if warnings.len() == 1 { "" } else { "s" },
            warnings.join("; ")
        );
    }

    if failed_required > 0 {
        println!();
        println!("Exit code: 1");
        Ok(false)
    } else {
        Ok(true)
    }
}

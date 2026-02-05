use anyhow::{bail, Result};
use colored::Colorize;
use std::path::PathBuf;

use crate::output;
use crate::resolver::{command_source, manual_source, static_source};
use crate::schema::validation::{load_and_validate_schema_file, SchemaValidation};
use crate::template;

pub struct PullOptions {
    pub schema_path: PathBuf,
    pub env_name: String,
    pub dry_run: bool,
    pub show_secrets: bool,
    pub force: bool,
    pub interactive: bool,
    pub destination_path: Option<PathBuf>,
    pub source_timeout: u64,
}

/// A resolved variable result.
enum ResolveResult {
    Success(String, String), // (var_name, value)
    Skipped(String, String), // (var_name, reason)
    Failed(String, String),  // (var_name, error)
}

fn print_labeled_multiline(indent: &str, label: &str, value: &str) {
    let value = value.trim();
    if value.is_empty() {
        return;
    }

    let lines: Vec<&str> = value.lines().collect();
    if lines.len() == 1 {
        println!("{}{}: {}", indent, label, lines[0]);
        return;
    }

    println!("{}{}:", indent, label);
    for line in lines {
        println!("{}  {}", indent, line);
    }
}

/// Run the `pull` command: resolve variables and write the .env file.
pub async fn run_pull(opts: PullOptions) -> Result<bool> {
    // Parse and validate schema
    let schema = match load_and_validate_schema_file(&opts.schema_path)? {
        SchemaValidation::Valid(schema) => schema,
        SchemaValidation::Invalid(errors) => {
            println!("{} Schema errors:", "✗".red());
            for error in &errors {
                println!("  - {}", error);
            }
            bail!("Schema validation failed. Fix errors before pulling.");
        }
    };

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
    let dest_path = if let Some(ref destination) = opts.destination_path {
        if destination.exists() && destination.is_dir() {
            let schema_dest = match schema.destination_for(&opts.env_name) {
                Some(p) => PathBuf::from(p),
                None => bail!(
                    "No destination defined for environment \"{}\" in metadata.destination.",
                    opts.env_name
                ),
            };
            let file_name = schema_dest.file_name().ok_or_else(|| {
                anyhow::anyhow!(
                    "Schema destination path \"{}\" does not have a file name.",
                    schema_dest.display()
                )
            })?;
            destination.join(file_name)
        } else {
            destination.clone()
        }
    } else {
        match schema.destination_for(&opts.env_name) {
            Some(p) => PathBuf::from(p),
            None => bail!(
                "No destination defined for environment \"{}\" in metadata.destination.",
                opts.env_name
            ),
        }
    };

    if dest_path.exists() && dest_path.is_dir() {
        bail!(
            "Destination path \"{}\" is a directory. Provide a file path.",
            dest_path.display()
        );
    }

    // Refuse to overwrite an existing destination without --force (even in dry-run).
    if dest_path.exists() && !opts.force {
        bail!(
            "Destination file \"{}\" already exists. Use --force to overwrite.",
            dest_path.display()
        );
    }

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
        let mut would_write_count = 0;
        let mut failed_required = 0;

        for (var_name, var) in &schema.variables {
            if !var.applies_to(&opts.env_name) {
                continue;
            }

            let source = match var.effective_source_for_env(&opts.env_name) {
                Some(s) => s,
                None => {
                    println!("  {}", var_name);
                    println!("    source:  <missing>");
                    println!(
                        "    error:   {}",
                        "No source configured for this variable/environment"
                    );
                    println!();
                    if var.required {
                        failed_required += 1;
                    }
                    continue;
                }
            };
            if source == "static" {
                static_manual_count += 1;
                let key = var.effective_key_for_env(var_name, &opts.env_name);
                let (value, ok) = match var.values_for_env(&opts.env_name) {
                    Some(values) => match static_source::resolve_static(
                        var_name,
                        &key,
                        values,
                        &opts.env_name,
                        env_config,
                    ) {
                        Ok(v) => {
                            let shown_value = if var.sensitive {
                                output::mask_value(&v, opts.show_secrets)
                            } else {
                                v
                            };
                            (shown_value, true)
                        }
                        Err(e) => {
                            if var.required {
                                failed_required += 1;
                            }
                            (format!("<error: {}>", e), false)
                        }
                    },
                    None => {
                        if var.required {
                            failed_required += 1;
                        }
                        ("<missing>".to_string(), false)
                    }
                };
                println!("  {}", var_name);
                println!("    source:  static");
                println!("    value:   {}", value);
                println!();
                if ok {
                    would_write_count += 1;
                }
            } else if source == "manual" {
                static_manual_count += 1;
                println!("  {}", var_name);
                if opts.interactive {
                    println!("    source:  manual (interactive prompt)");
                    would_write_count += 1;
                } else {
                    println!("    source:  manual (skipped; use --interactive to prompt)");
                }
                if let Some(instructions) = &var.source_instructions {
                    let key = var.effective_key_for_env(var_name, &opts.env_name);
                    let ctx = template::build_context(&opts.env_name, env_config, &key);
                    let expanded = template::expand_template_best_effort(instructions, &ctx);
                    print_labeled_multiline("    ", "instructions", &expanded);
                }
                println!();
            } else {
                match schema.sources.get(source) {
                    Some(src) => {
                        let key = var.effective_key_for_env(var_name, &opts.env_name);
                        let cmd = match command_source::build_command(
                            &src.command,
                            var_name,
                            Some(&key),
                            &opts.env_name,
                            env_config,
                        ) {
                            Ok(cmd) => {
                                command_count += 1;
                                would_write_count += 1;
                                cmd
                            }
                            Err(e) => {
                                if var.required {
                                    failed_required += 1;
                                }
                                format!("<error: {}>", e)
                            }
                        };

                        println!("  {}", var_name);
                        println!("    source:  {}", source);
                        println!("    command: {}", cmd);
                        println!();
                    }
                    None => {
                        println!("  {}", var_name);
                        println!("    source:  {}", source);
                        println!("    command: <missing>");
                        println!("    error:   Source \"{}\" is not defined in sources.", source);
                        println!();
                        if var.required {
                            failed_required += 1;
                        }
                    }
                }
            }
        }

        if would_write_count > 0 {
            println!(
                "{} variable{} would be written to {}",
                would_write_count,
                if would_write_count == 1 { "" } else { "s" },
                dest_path.display()
            );
        } else {
            println!("No variables would be resolved. Output file would not be written.");
        }
        println!(
            "{} command{} would be executed ({} static/manual)",
            command_count,
            if command_count == 1 { "" } else { "s" },
            static_manual_count
        );
        if failed_required > 0 {
            println!();
            println!("Exit code: 1");
            return Ok(false);
        }

        return Ok(true);
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
    let mut manual_vars: Vec<(String, String, String, Option<String>, bool, bool)> = Vec::new(); // (var_name, key, description, instructions, required, sensitive)

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
            let key = var.effective_key_for_env(var_name, &opts.env_name);
            match var.values_for_env(&opts.env_name) {
                Some(values) => match static_source::resolve_static(
                    var_name,
                    &key,
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
            let key = var.effective_key_for_env(var_name, &opts.env_name);
            manual_vars.push((
                var_name.to_string(),
                key,
                var.description.clone(),
                var.source_instructions.clone(),
                var.required,
                var.sensitive,
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
        let timeout = opts.source_timeout;
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
    for (var_name, key, description, instructions, required, sensitive) in manual_vars {
        match manual_source::resolve_manual(manual_source::ManualResolveOptions {
            var_name: &var_name,
            key: &key,
            description: &description,
            source_instructions: instructions.as_deref(),
            env_name: &opts.env_name,
            env_config,
            sensitive,
            non_interactive: !opts.interactive,
        }) {
            Ok(Some(value)) => {
                all_results.push(ResolveResult::Success(var_name, value));
            }
            Ok(None) => {
                // Skipped in non-interactive mode
                all_results.push(ResolveResult::Skipped(
                    var_name,
                    "skipped (non-interactive mode)".to_string(),
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

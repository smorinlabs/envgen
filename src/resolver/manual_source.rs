use anyhow::Result;
use dialoguer::Input;
use std::collections::HashMap;

use crate::template;

/// Prompt the user for a manual variable value.
/// Returns None if non-interactive mode is enabled.
pub fn resolve_manual(
    var_name: &str,
    description: &str,
    source_instructions: Option<&str>,
    env_name: &str,
    env_config: &std::collections::BTreeMap<String, String>,
    non_interactive: bool,
) -> Result<Option<String>> {
    if non_interactive {
        return Ok(None);
    }

    // Build context for expanding template placeholders in instructions
    let ctx = template::build_context(env_name, env_config, var_name);

    println!();
    println!("  Variable: {}", var_name);
    println!("  Description: {}", description);

    if let Some(instructions) = source_instructions {
        let expanded = expand_instructions(instructions, &ctx);
        println!("  Instructions: {}", expanded.trim());
    }

    println!();

    let value: String = Input::new()
        .with_prompt(format!("  Enter value for {}", var_name))
        .interact_text()?;

    Ok(Some(value))
}

fn expand_instructions(instructions: &str, ctx: &HashMap<String, String>) -> String {
    // Best-effort expansion — don't fail if a placeholder can't be resolved
    template::expand_template(instructions, ctx).unwrap_or_else(|_| instructions.to_string())
}

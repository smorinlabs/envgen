use anyhow::Result;
use dialoguer::{Input, Password};
use std::collections::{BTreeMap, HashMap};

use crate::template;

pub struct ManualResolveOptions<'a> {
    pub var_name: &'a str,
    pub key: &'a str,
    pub description: &'a str,
    pub source_instructions: Option<&'a str>,
    pub env_name: &'a str,
    pub env_config: &'a BTreeMap<String, String>,
    pub sensitive: bool,
    pub non_interactive: bool,
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

/// Prompt the user for a manual variable value.
/// Returns None if non-interactive mode is enabled.
pub fn resolve_manual(opts: ManualResolveOptions<'_>) -> Result<Option<String>> {
    if opts.non_interactive {
        return Ok(None);
    }

    // Build context for expanding template placeholders in instructions
    let ctx = template::build_context(opts.env_name, opts.env_config, opts.key);

    println!();
    println!("  Variable: {}", opts.var_name);
    println!("  Description: {}", opts.description);

    if let Some(instructions) = opts.source_instructions {
        let expanded = expand_instructions(instructions, &ctx);
        print_labeled_multiline("  ", "Instructions", &expanded);
    }

    println!();

    let prompt = format!("  Enter value for {}", opts.var_name);
    let value: String = if opts.sensitive {
        Password::new().with_prompt(prompt).interact()?
    } else {
        Input::new().with_prompt(prompt).interact_text()?
    };

    Ok(Some(value))
}

fn expand_instructions(instructions: &str, ctx: &HashMap<String, String>) -> String {
    // Best-effort expansion — don't fail if a placeholder can't be resolved
    template::expand_template(instructions, ctx).unwrap_or_else(|_| instructions.to_string())
}

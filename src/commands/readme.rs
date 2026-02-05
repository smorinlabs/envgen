use anyhow::Result;

const README_MARKDOWN: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"));

/// Run the `readme` command: print the embedded README.md to stdout.
pub fn run_readme() -> Result<()> {
    print!("{}", README_MARKDOWN);
    Ok(())
}

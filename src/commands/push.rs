use anyhow::Result;
use std::path::PathBuf;

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

/// Run the `push` command. Returns the process exit code.
pub async fn run_push(_opts: PushOptions) -> Result<i32> {
    anyhow::bail!("push: not yet implemented");
}

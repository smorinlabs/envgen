mod commands;
mod output;
mod resolver;
mod schema;
mod template;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process;

#[derive(Parser)]
#[command(
    name = "envgen",
    version,
    about = "Generate .env files from declarative YAML schemas"
)]
struct Cli {
    /// Path to schema YAML file
    #[arg(short, long, global = true)]
    schema: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Resolve all variables and write the destination .env file
    Pull {
        /// Path to schema YAML file
        #[arg(short, long)]
        schema: Option<PathBuf>,

        /// Target environment (defaults to "local")
        #[arg(short, long, default_value = "local")]
        env: String,

        /// Print what would be written without executing anything
        #[arg(short = 'n', long)]
        dry_run: bool,

        /// Show actual sensitive values in dry-run output
        #[arg(long)]
        unmask: bool,

        /// Overwrite the destination file if it already exists
        #[arg(short, long)]
        force: bool,

        /// Skip manual source variables instead of prompting
        #[arg(long)]
        non_interactive: bool,

        /// Override the destination path from the schema
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Timeout in seconds for source commands (default: 30)
        #[arg(long, default_value = "30")]
        timeout: u64,
    },

    /// Validate a schema file for correctness
    Check {
        /// Path to schema YAML file
        #[arg(short, long)]
        schema: Option<PathBuf>,
    },

    /// Display a table of all variables defined in the schema
    List {
        /// Path to schema YAML file
        #[arg(short, long)]
        schema: Option<PathBuf>,

        /// Filter to variables applicable to a specific environment
        #[arg(short, long)]
        env: Option<String>,

        /// Output format: table (default) or json
        #[arg(long, default_value = "table")]
        format: String,
    },
}

fn resolve_schema_path(global: &Option<PathBuf>, local: &Option<PathBuf>) -> PathBuf {
    // Local (subcommand) flag takes precedence over global
    local
        .clone()
        .or_else(|| global.clone())
        .unwrap_or_else(|| {
            eprintln!("Error: --schema is required. Specify the path to a YAML schema file.");
            process::exit(1);
        })
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let exit_code = match cli.command {
        Commands::Check { ref schema } => {
            let schema_path = resolve_schema_path(&cli.schema, schema);
            match commands::check::run_check(&schema_path) {
                Ok(true) => 0,
                Ok(false) => 1,
                Err(e) => {
                    eprintln!("Error: {:#}", e);
                    1
                }
            }
        }
        Commands::List {
            ref schema,
            ref env,
            ref format,
        } => {
            let schema_path = resolve_schema_path(&cli.schema, schema);
            let fmt = match commands::list::ListFormat::from_str(format) {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    process::exit(1);
                }
            };
            match commands::list::run_list(&schema_path, env.as_deref(), fmt) {
                Ok(()) => 0,
                Err(e) => {
                    eprintln!("Error: {:#}", e);
                    1
                }
            }
        }
        Commands::Pull {
            ref schema,
            ref env,
            dry_run,
            unmask,
            force,
            non_interactive,
            ref output,
            timeout,
        } => {
            let schema_path = resolve_schema_path(&cli.schema, schema);
            let opts = commands::pull::PullOptions {
                schema_path,
                env_name: env.clone(),
                dry_run,
                unmask,
                force,
                non_interactive,
                output_path: output.clone(),
                timeout,
            };
            match commands::pull::run_pull(opts).await {
                Ok(true) => 0,
                Ok(false) => 1,
                Err(e) => {
                    eprintln!("Error: {:#}", e);
                    1
                }
            }
        }
    };

    process::exit(exit_code);
}

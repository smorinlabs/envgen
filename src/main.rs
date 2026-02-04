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
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Resolve all variables and write the destination .env file
    Pull {
        /// Path to envgen YAML config file
        #[arg(short = 'c', long)]
        config: PathBuf,

        /// Target environment
        #[arg(short, long, default_value = "local")]
        env: String,

        /// Print what would be written without executing anything
        #[arg(short = 'n', long)]
        dry_run: bool,

        /// Show actual sensitive values in dry-run output
        #[arg(long, requires = "dry_run")]
        show_secrets: bool,

        /// Overwrite the destination file if it already exists
        #[arg(short, long)]
        force: bool,

        /// Prompt for manual source variables instead of skipping them
        #[arg(short, long)]
        interactive: bool,

        /// Override the destination path from the schema
        #[arg(short = 'd', long)]
        destination: Option<PathBuf>,

        /// Timeout in seconds for each source command
        #[arg(long, default_value = "30")]
        source_timeout: u64,
    },

    /// Create a sample schema file
    Init {
        /// Output path (file or directory)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Overwrite the destination file if it already exists
        #[arg(short, long)]
        force: bool,

        /// Suppress success output
        #[arg(short, long)]
        quiet: bool,
    },

    /// Validate a schema file for correctness
    Check {
        /// Path to envgen YAML config file
        #[arg(short = 'c', long)]
        config: PathBuf,
    },

    /// Display a table of all variables defined in the schema
    List {
        /// Path to envgen YAML config file
        #[arg(short = 'c', long)]
        config: PathBuf,

        /// Filter to variables applicable to a specific environment
        #[arg(short, long)]
        env: Option<String>,

        /// Output format: table or json
        #[arg(long, default_value = "table")]
        format: String,
    },

    /// Generate Markdown documentation for a schema file
    Docs {
        /// Path to envgen YAML config file
        #[arg(short = 'c', long)]
        config: PathBuf,

        /// Filter to variables applicable to a specific environment
        #[arg(short, long)]
        env: Option<String>,
    },

    /// Export the embedded JSON Schema used to validate envgen YAML schemas
    Schema {
        /// Output path (file or directory)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Overwrite the destination file if it already exists
        #[arg(short, long)]
        force: bool,

        /// Suppress success output
        #[arg(short, long)]
        quiet: bool,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let exit_code = match cli.command {
        Commands::Init {
            ref output,
            force,
            quiet,
        } => {
            let opts = commands::init::InitOptions {
                output_path: output.clone(),
                force,
                quiet,
            };
            match commands::init::run_init(opts) {
                Ok(()) => 0,
                Err(e) => {
                    eprintln!("Error: {:#}", e);
                    1
                }
            }
        }
        Commands::Check { ref config } => match commands::check::run_check(config) {
            Ok(true) => 0,
            Ok(false) => 1,
            Err(e) => {
                eprintln!("Error: {:#}", e);
                1
            }
        },
        Commands::List {
            ref config,
            ref env,
            ref format,
        } => {
            let fmt = match commands::list::ListFormat::from_str(format) {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    process::exit(1);
                }
            };
            match commands::list::run_list(config, env.as_deref(), fmt) {
                Ok(()) => 0,
                Err(e) => {
                    eprintln!("Error: {:#}", e);
                    1
                }
            }
        }
        Commands::Docs {
            ref config,
            ref env,
        } => match commands::docs::run_docs(config, env.as_deref()) {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("Error: {:#}", e);
                1
            }
        },
        Commands::Pull {
            ref config,
            ref env,
            dry_run,
            show_secrets,
            force,
            interactive,
            ref destination,
            source_timeout,
        } => {
            let opts = commands::pull::PullOptions {
                schema_path: config.clone(),
                env_name: env.clone(),
                dry_run,
                show_secrets,
                force,
                interactive,
                destination_path: destination.clone(),
                source_timeout,
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
        Commands::Schema {
            ref output,
            force,
            quiet,
        } => {
            if output.as_deref() == Some(PathBuf::from("-").as_ref()) {
                match commands::schema::run_schema_print() {
                    Ok(()) => 0,
                    Err(e) => {
                        eprintln!("Error: {:#}", e);
                        1
                    }
                }
            } else {
                let opts = commands::schema::SchemaExportOptions {
                    output_path: output.clone(),
                    force,
                    quiet,
                };
                match commands::schema::run_schema_export(opts) {
                    Ok(()) => 0,
                    Err(e) => {
                        eprintln!("Error: {:#}", e);
                        1
                    }
                }
            }
        }
    };

    process::exit(exit_code);
}

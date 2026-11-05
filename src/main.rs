mod compiler;
mod config;
mod gemma;
mod interpreter;
mod repl;

use clap::{Parser, Subcommand};
use colored::Colorize;
use config::{BackendKind, Config};
use interpreter::Interpreter;
use repl::Repl;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "ailang",
    author = "Ailang Contributors",
    version = "0.1.0",
    about = "An interpreted language powered by LLMs (Qwen2.5-Coder / Gemma) that converts instructions to Rust and runs them JIT",
    long_about = "Ailang lets you write programs and instructions in natural language or DSL,\n\
                  uses local or cloud LLMs (Qwen2.5-Coder / Gemma) to translate them into safe, high-performance Rust,\n\
                  and compiles and executes them on the fly with automatic self-repair."
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Select LLM backend: auto, ollama, google, openai, cmd, mock
    #[arg(short, long, global = true)]
    backend: Option<String>,

    /// Model identifier (e.g. qwen2.5-coder:0.5b, gemma2, gemma-2-27b-it)
    #[arg(short, long, global = true)]
    model: Option<String>,

    /// API key for cloud Gemma providers (Google AI Studio, OpenAI-compatible)
    #[arg(long, global = true)]
    api_key: Option<String>,

    /// Custom endpoint URL (for Ollama or OpenAI-compatible server)
    #[arg(long, global = true)]
    endpoint: Option<String>,

    /// Timeout in seconds for LLM generation
    #[arg(long, global = true)]
    timeout: Option<u64>,

    /// Print generated Rust code before running
    #[arg(long, global = true)]
    show_rust: bool,

    /// Verbose output with compilation and execution timing
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Keep temporary binaries and build files
    #[arg(long, global = true)]
    keep_temp: bool,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Start interactive Ailang REPL (default)
    Repl,

    /// Run an Ailang script file (.ail)
    Run {
        /// Path to the .ail script file
        file: PathBuf,

        /// Arguments passed to the executed program
        #[arg(last = true)]
        args: Vec<String>,
    },

    /// Evaluate a single instruction directly from the command line
    Eval {
        /// Instruction to evaluate
        instruction: String,

        /// Arguments passed to the executed program
        #[arg(last = true)]
        args: Vec<String>,
    },

    /// Transpile Ailang instruction or script into a standalone Rust file
    Transpile {
        /// Input script file or instruction
        input: String,

        /// Output Rust file (.rs)
        #[arg(short, long, default_value = "output.rs")]
        output: PathBuf,
    },

    /// Verify environment: checks rustc and Gemma backend connectivity
    Check,

    /// View or inspect current Ailang configuration
    Config,
}

fn apply_cli_overrides(mut config: Config, cli: &Cli) -> Config {
    if let Some(ref b) = cli.backend {
        config.backend = match b.to_lowercase().as_str() {
            "ollama" => BackendKind::Ollama,
            "google" | "gemini" => BackendKind::Google,
            "openai" | "vllm" => BackendKind::OpenAi,
            "cmd" | "command" => BackendKind::Command,
            "mock" => BackendKind::Mock,
            _ => BackendKind::Auto,
        };
    }

    if let Some(ref m) = cli.model {
        config.model = m.clone();
    }

    if let Some(ref k) = cli.api_key {
        config.api_key = Some(k.clone());
    }

    if let Some(ref ep) = cli.endpoint {
        config.endpoint = Some(ep.clone());
    }

    if let Some(t) = cli.timeout {
        config.timeout_secs = t;
    }

    if cli.show_rust {
        config.show_rust = true;
    }

    if cli.verbose {
        config.verbose = true;
    }

    if cli.keep_temp {
        config.keep_temp = true;
    }

    config
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let config = Config::load();
    let config = apply_cli_overrides(config, &cli);

    match cli.command.unwrap_or(Commands::Repl) {
        Commands::Repl => {
            let interpreter = Interpreter::new(config);
            let mut repl = Repl::new(interpreter);
            repl.start()?;
        }
        Commands::Run { file, args } => {
            let mut interpreter = Interpreter::new(config);
            match interpreter.execute_file(&file, &args) {
                Ok(result) => {
                    if !result.stdout.is_empty() {
                        print!("{}", result.stdout);
                    }
                    if !result.stderr.is_empty() {
                        eprint!("{}", result.stderr.yellow());
                    }
                    if let Some(code) = result.exit_code {
                        if code != 0 {
                            std::process::exit(code);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("{}: {}", "Execution failed".red().bold(), e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Eval { instruction, args } => {
            let mut interpreter = Interpreter::new(config);
            match interpreter.execute_instruction(&instruction, &args) {
                Ok(result) => {
                    if !result.stdout.is_empty() {
                        print!("{}", result.stdout);
                    }
                    if !result.stderr.is_empty() {
                        eprint!("{}", result.stderr.yellow());
                    }
                    if let Some(code) = result.exit_code {
                        if code != 0 {
                            std::process::exit(code);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("{}: {}", "Execution failed".red().bold(), e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Transpile { input, output } => {
            let interpreter = Interpreter::new(config);
            let input_content = if std::path::Path::new(&input).exists() {
                std::fs::read_to_string(&input)?
            } else {
                input
            };

            println!("{} Translating instruction with Gemma...", "⏳".cyan());
            interpreter.transpile_to_file(&input_content, &output)?;
            println!("{} Standalone Rust code written to {}", "✓".green().bold(), output.display());
        }
        Commands::Check => {
            println!("{}", "=== Ailang Environment Diagnostics ===".cyan().bold());

            // 1. Check rustc
            match compiler::RustRunner::check_rustc() {
                Ok(ver) => println!("{} Rust compiler: {}", "✓".green().bold(), ver),
                Err(e) => println!("{} Rust compiler: {}", "✗".red().bold(), e),
            }

            // 2. Check Gemma Backend
            let interpreter = Interpreter::new(config.clone());
            println!("{} Configured backend: {}", "ℹ".blue().bold(), interpreter.backend.name());
            println!("{} Model: {}", "ℹ".blue().bold(), config.model);

            match interpreter.backend.check_health() {
                Ok(true) => println!("{} Backend health: Healthy & reachable", "✓".green().bold()),
                Ok(false) => println!("{} Backend health: Unhealthy or unreachable", "✗".red().bold()),
                Err(e) => println!("{} Backend health check failed: {}", "✗".red().bold(), e),
            }
        }
        Commands::Config => {
            println!("{}", "=== Current Ailang Configuration ===".cyan().bold());
            println!("{}", serde_json::to_string_pretty(&config)?);
            if let Some(path) = Config::config_file_path() {
                println!("\nConfig path: {}", path.display());
            }
        }
    }

    Ok(())
}

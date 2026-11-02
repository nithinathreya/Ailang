use crate::interpreter::Interpreter;
use colored::Colorize;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use std::path::PathBuf;

pub struct Repl {
    interpreter: Interpreter,
    history_file: Option<PathBuf>,
}

impl Repl {
    pub fn new(interpreter: Interpreter) -> Self {
        let history_file = std::env::var("HOME").ok().map(|h| {
            let dir = PathBuf::from(h).join(".ailang");
            let _ = std::fs::create_dir_all(&dir);
            dir.join("history.txt")
        });

        Self {
            interpreter,
            history_file,
        }
    }

    pub fn start(&mut self) -> anyhow::Result<()> {
        let mut rl = DefaultEditor::new()?;

        if let Some(ref path) = self.history_file {
            let _ = rl.load_history(path);
        }

        self.print_welcome();

        loop {
            let prompt = format!("{} ", "ailang>".bright_magenta().bold());
            let readline = rl.readline(&prompt);

            match readline {
                Ok(line) => {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }

                    let _ = rl.add_history_entry(trimmed);

                    if trimmed.starts_with(':') {
                        if self.handle_command(trimmed) {
                            break;
                        }
                        continue;
                    }

                    self.run_instruction(trimmed);
                }
                Err(ReadlineError::Interrupted) => {
                    println!("{}", "^C (type :exit or Ctrl-D to exit)".dimmed());
                }
                Err(ReadlineError::Eof) => {
                    println!("\n{}", "Goodbye!".green());
                    break;
                }
                Err(err) => {
                    eprintln!("{}: {:?}", "REPL Error".red().bold(), err);
                    break;
                }
            }
        }

        if let Some(ref path) = self.history_file {
            let _ = rl.save_history(path);
        }

        Ok(())
    }

    fn print_welcome(&self) {
        println!("{}", "=========================================================".cyan());
        println!(
            "{}",
            "  AILang: Google Gemma -> Native Rust Interpreter  ".bright_white().bold()
        );
        println!("{}", "=========================================================".cyan());
        println!(
            "{} {}",
            "Backend:".bold(),
            self.interpreter.backend.name().green()
        );
        println!(
            "{} {}",
            "Model:  ".bold(),
            self.interpreter.config.model.yellow()
        );
        println!(
            "{} Type instructions or code. Enter {} for commands, {} to quit.\n",
            "Tip:".dimmed(),
            ":help".cyan(),
            ":exit".cyan()
        );
    }

    fn handle_command(&mut self, cmd: &str) -> bool {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        let name = parts[0];

        match name {
            ":exit" | ":quit" | ":q" => {
                println!("{}", "Goodbye!".green());
                return true;
            }
            ":help" | ":h" => {
                println!("{}", "REPL Commands:".bold().underline());
                println!("  {}         - Display this help message", ":help".cyan());
                println!("  {}         - View session history, variables, and definitions", ":state".cyan());
                println!("  {}          - Toggle showing generated Rust source code", ":rust".cyan());
                println!("  {}       - Toggle verbose timing and compilation info", ":verbose".cyan());
                println!("  {}         - Clear session context and definitions", ":reset".cyan());
                println!("  {}         - Clear the terminal screen", ":clear".cyan());
                println!("  {} <file>  - Export the last generated Rust code to a file", ":export".cyan());
                println!("  {}          - Exit the REPL", ":exit".cyan());
            }
            ":rust" => {
                self.interpreter.config.show_rust = !self.interpreter.config.show_rust;
                println!(
                    "Show Rust code: {}",
                    if self.interpreter.config.show_rust {
                        "ON".green().bold()
                    } else {
                        "OFF".yellow()
                    }
                );
            }
            ":verbose" => {
                self.interpreter.config.verbose = !self.interpreter.config.verbose;
                println!(
                    "Verbose mode: {}",
                    if self.interpreter.config.verbose {
                        "ON".green().bold()
                    } else {
                        "OFF".yellow()
                    }
                );
            }
            ":reset" => {
                self.interpreter.reset_context();
                println!("{}", "Session state reset.".yellow());
            }
            ":clear" => {
                print!("\x1B[2J\x1B[1;1H");
            }
            ":state" => {
                println!("{}", "=== Current Session State ===".cyan().bold());
                println!("Definitions: {}", self.interpreter.context.definitions.len());
                for (idx, def) in self.interpreter.context.definitions.iter().enumerate() {
                    println!("[{}] {}", idx + 1, def.lines().next().unwrap_or(""));
                }
                println!("Execution history entries: {}", self.interpreter.context.history.len());
                println!("Active backend: {}", self.interpreter.backend.name());
            }
            ":export" => {
                if parts.len() < 2 {
                    println!("{}", "Usage: :export <filename.rs>".red());
                } else if let Some(last) = self.interpreter.context.history.last() {
                    let out_path = parts[1];
                    if let Err(e) = std::fs::write(out_path, &last.generated_rust) {
                        println!("{}: {}", "Export failed".red(), e);
                    } else {
                        println!("{} Saved Rust code to {}", "✓".green(), out_path.bold());
                    }
                } else {
                    println!("{}", "No code has been generated yet in this session.".yellow());
                }
            }
            _ => {
                println!(
                    "{} Unknown command '{}'. Type :help for commands.",
                    "✗".red(),
                    name
                );
            }
        }

        false
    }

    fn run_instruction(&mut self, instruction: &str) {
        let spinner = "⚡ Translating with Gemma & Compiling to Rust...".dimmed();
        println!("{}", spinner);

        match self.interpreter.execute_instruction(instruction, &[]) {
            Ok(result) => {
                if !result.stdout.is_empty() {
                    print!("{}", result.stdout);
                }
                if !result.stderr.is_empty() {
                    eprint!("{}", result.stderr.yellow());
                }

                if self.interpreter.config.verbose {
                    println!(
                        "{}",
                        format!(
                            "[Compiled in {}ms, Executed in {}ms]",
                            result.compile_duration_ms, result.run_duration_ms
                        )
                        .cyan()
                        .dimmed()
                    );
                }
            }
            Err(err) => {
                eprintln!("{}: {}", "Error".red().bold(), err);
            }
        }
    }
}

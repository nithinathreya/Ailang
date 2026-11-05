pub mod ast;
pub mod environment;
pub mod evaluator;
pub mod lexer;
pub mod parser;
pub mod semantic;
pub mod token;
pub mod value;

use crate::compiler::{CompilationError, ExecutionResult, RustRunner};
use crate::config::Config;
use crate::gemma::{create_backend, extract_rust_code, GemmaBackend};
use anyhow::{anyhow, Context, Result};
use colored::Colorize;
use std::collections::HashSet;
use std::path::Path;
use std::time::Instant;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct InstructionRecord {
    pub instruction: String,
    pub generated_rust: String,
    pub success: bool,
}

#[derive(Debug, Clone)]
pub struct SessionContext {
    pub imports: HashSet<String>,
    pub definitions: Vec<String>,
    pub history: Vec<InstructionRecord>,
}

impl Default for SessionContext {
    fn default() -> Self {
        let mut imports = HashSet::new();
        imports.insert("use std::collections::*;".to_string());
        imports.insert("use std::fmt::Write as _FmtWrite;".to_string());
        imports.insert("use std::io::Write as _IoWrite;".to_string());

        Self {
            imports,
            definitions: Vec::new(),
            history: Vec::new(),
        }
    }
}

pub struct Interpreter {
    pub config: Config,
    pub backend: Box<dyn GemmaBackend>,
    pub runner: RustRunner,
    pub context: SessionContext,
    pub evaluator: evaluator::Evaluator,
}

impl Interpreter {
    pub fn new(config: Config) -> Self {
        let backend = create_backend(&config);
        let runner = RustRunner::new(config.keep_temp);
        let context = SessionContext::default();
        let evaluator = evaluator::Evaluator::new();

        Self {
            config,
            backend,
            runner,
            context,
            evaluator,
        }
    }

    pub fn reset_context(&mut self) {
        self.context = SessionContext::default();
        self.evaluator = evaluator::Evaluator::new();
    }

    /// Tokenize source code using the native Lexer
    pub fn tokenize(&self, source: &str) -> Result<Vec<token::Token>> {
        let mut lexer = lexer::Lexer::new(source);
        lexer.tokenize()
    }

    /// Parse source code into an Abstract Syntax Tree (AST) using the native recursive descent parser
    pub fn parse(&self, source: &str) -> Result<Vec<ast::Stmt>> {
        let tokens = self.tokenize(source)?;
        let mut parser = parser::Parser::new(tokens);
        parser.parse()
    }

    /// Check if input is valid native Ailang syntax
    pub fn try_parse_native(input: &str) -> Option<Vec<ast::Stmt>> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return None;
        }

        let has_code_indicators = trimmed.starts_with("let ")
            || trimmed.starts_with("fn ")
            || trimmed.starts_with("while ")
            || trimmed.starts_with("for ")
            || trimmed.starts_with("if ")
            || trimmed.starts_with("print ")
            || trimmed.contains(';')
            || trimmed.contains('{')
            || trimmed.contains(" = ")
            || trimmed.contains("==");

        if !has_code_indicators {
            return None;
        }

        let mut lexer = lexer::Lexer::new(trimmed);
        let tokens = lexer.tokenize().ok()?;
        if tokens.len() <= 1 {
            return None;
        }

        let mut parser = parser::Parser::new(tokens);
        let mut stmts = parser.parse().ok()?;

        let mut semantic_analyzer = semantic::SemanticAnalyzer::new();
        if semantic_analyzer.analyze(&mut stmts).is_ok() {
            Some(stmts)
        } else {
            None
        }
    }

    /// Execute native Ailang AST statements
    pub fn execute_native(&mut self, mut stmts: Vec<ast::Stmt>) -> Result<ExecutionResult> {
        let start = Instant::now();

        // Run semantic analysis & constant folding optimization
        let mut analyzer = semantic::SemanticAnalyzer::new();
        analyzer.analyze(&mut stmts)?;

        let val_opt = self.evaluator.evaluate_program(&stmts)?;
        let run_duration = start.elapsed().as_millis();

        let mut stdout = if !self.evaluator.output_buffer.is_empty() {
            let out = self.evaluator.output_buffer.join("\n");
            self.evaluator.output_buffer.clear();
            if !out.ends_with('\n') {
                format!("{}\n", out)
            } else {
                out
            }
        } else {
            String::new()
        };

        // If an expression was evaluated without print, show its value
        if stdout.is_empty() {
            if let Some(val) = val_opt {
                if val != value::Value::Nil {
                    stdout = format!("{}\n", val);
                }
            }
        }

        Ok(ExecutionResult {
            success: true,
            stdout,
            stderr: String::new(),
            exit_code: Some(0),
            compile_duration_ms: 0,
            run_duration_ms: run_duration,
            rust_source: "// Executed via native Ailang AST tree-walker\n".to_string(),
            binary_path: None,
        })
    }

    /// Build system prompt guiding the LLM to produce optimal Rust code
    fn build_system_prompt(&self) -> String {
        let mut prompt = String::from(
            r#"You are an expert Rust programmer. Convert the user instruction into a complete, runnable, and correct Rust program with `fn main()`.
Rules:
1. Output ONLY valid Rust code inside a ```rust ... ``` block. No conversational text.
2. Always specify explicit types for numbers and variables (e.g. `1i32..=5i32`, `let x: u64 = ...`) to prevent type inference ambiguity.
3. Idioms: for string reversal use `s.chars().rev().collect::<String>()`; for prime checking write a helper function `fn is_prime(n: u64) -> bool`.
4. Make sure all results are printed to stdout via println!.
5. The code will be compiled with `rustc --edition 2024` as a standalone file. Only use the Rust standard library (`std::*`), such as `std::net::TcpListener` for HTTP and network servers, `std::io`, and `std::thread`. Do not use external crates.
"#,
        );

        if !self.context.definitions.is_empty() {
            prompt.push_str("\nExisting structs, types, or helper functions in the current session:\n");
            for def in &self.context.definitions {
                prompt.push_str(def);
                prompt.push('\n');
            }
        }

        if !self.context.imports.is_empty() {
            prompt.push_str("\nSession imports:\n");
            for imp in &self.context.imports {
                prompt.push_str(imp);
                prompt.push('\n');
            }
        }

        prompt
    }

    /// Translate instruction into Rust code using Gemma
    pub fn translate_instruction(&self, instruction: &str) -> Result<String> {
        let system_prompt = self.build_system_prompt();
        let raw_response = self.backend.translate(instruction, &system_prompt)?;
        let code = extract_rust_code(&raw_response);
        Ok(code)
    }

    /// Self-correcting loop: attempt compilation, and if it fails, ask Gemma to fix the code
    pub fn compile_and_run_with_repair(
        &self,
        instruction: &str,
        mut rust_code: String,
        args: &[String],
    ) -> Result<ExecutionResult> {
        let mut attempts = 0;
        let max_attempts = self.config.max_retries;

        loop {
            if self.config.show_rust || self.config.verbose {
                println!("{}", "--- [Generated Rust Code] ---".blue().bold());
                println!("{}", rust_code);
                println!("{}", "-----------------------------".blue().bold());
            }

            match self.runner.run_source(&rust_code, args) {
                Ok(result) => {
                    return Ok(result);
                }
                Err(err) => {
                    attempts += 1;
                    if attempts > max_attempts {
                        return Err(err);
                    }

                    let compiler_stderr = if let Some(comp_err) = err.downcast_ref::<CompilationError>() {
                        comp_err.stderr.clone()
                    } else {
                        err.to_string()
                    };

                    if self.config.verbose {
                        println!(
                            "{}",
                            format!(
                                "⚡ Compilation attempt {} failed. Asking Gemma to repair the code...",
                                attempts
                            )
                            .yellow()
                        );
                    }

                    let repair_prompt = format!(
                        "The Rust code failed to compile with `rustc`:\n\n\
                        ```\n{}\n```\n\n\
                        Failing Rust code:\n```rust\n{}\n```\n\n\
                        Original instruction: {}\n\n\
                        Fix the compiler error completely. Output ONLY the fixed working Rust code inside ```rust ... ```.",
                        compiler_stderr, rust_code, instruction
                    );

                    let system_prompt = self.build_system_prompt();
                    let repair_response = self.backend.translate(&repair_prompt, &system_prompt)?;
                    rust_code = extract_rust_code(&repair_response);
                }
            }
        }
    }

    /// Execute an instruction, using native AST evaluation if valid code, or JIT compilation for natural language
    pub fn execute_instruction(&mut self, instruction: &str, args: &[String]) -> Result<ExecutionResult> {
        let trimmed = instruction.trim();
        if trimmed.is_empty() {
            return Err(anyhow!("Empty instruction"));
        }

        // 1. Try native AST execution first
        if let Some(stmts) = Self::try_parse_native(trimmed) {
            if self.config.verbose {
                println!("{}", "[Executing via Native Ailang Interpreter]".green().bold());
            }
            return self.execute_native(stmts);
        }

        // 2. Fall back to LLM translation & JIT Rust compilation
        let is_type_def = trimmed.starts_with("struct ")
            || trimmed.starts_with("enum ")
            || trimmed.starts_with("type ")
            || trimmed.starts_with("fn ");

        let rust_code = self.translate_instruction(trimmed)?;

        if is_type_def {
            self.context.definitions.push(rust_code.clone());
        }

        let exec_result = self.compile_and_run_with_repair(trimmed, rust_code.clone(), args)?;

        self.context.history.push(InstructionRecord {
            instruction: trimmed.to_string(),
            generated_rust: rust_code,
            success: exec_result.success,
        });

        Ok(exec_result)
    }

    /// Execute an entire script file written in Ailang
    pub fn execute_file(&mut self, path: &Path, args: &[String]) -> Result<ExecutionResult> {
        let script_content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read script file: {}", path.display()))?;

        if self.config.verbose {
            println!(
                "{}",
                format!("Executing script: {} ({} bytes)", path.display(), script_content.len()).cyan()
            );
        }

        self.execute_instruction(&script_content, args)
    }

    /// Transpile instruction or script directly to standalone Rust file
    pub fn transpile_to_file(&self, instruction: &str, output_path: &Path) -> Result<()> {
        let rust_code = self.translate_instruction(instruction)?;
        std::fs::write(output_path, &rust_code)
            .with_context(|| format!("Failed to write transpile output to {}", output_path.display()))?;
        Ok(())
    }
}

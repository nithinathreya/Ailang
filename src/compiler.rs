use anyhow::{Context, Result, anyhow};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;
use tempfile::Builder;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ExecutionResult {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub compile_duration_ms: u128,
    pub run_duration_ms: u128,
    pub rust_source: String,
    pub binary_path: Option<PathBuf>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CompilationError {
    pub stderr: String,
    pub rust_source: String,
}

impl std::fmt::Display for CompilationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Rustc compilation failed:\n{}", self.stderr)
    }
}

impl std::error::Error for CompilationError {}

pub struct RustRunner {
    pub keep_temp: bool,
    pub opt_level: String,
}

impl RustRunner {
    pub fn new(keep_temp: bool) -> Self {
        Self {
            keep_temp,
            opt_level: "1".to_string(), // -C opt-level=1 strikes a good balance between compile speed and execution speed
        }
    }

    /// Compile a Rust source string into a standalone binary using `rustc`
    pub fn compile(&self, source_code: &str, output_bin: &Path) -> Result<(), CompilationError> {
        let temp_dir = Builder::new()
            .prefix("ailang_src_")
            .tempdir()
            .map_err(|e| CompilationError {
                stderr: format!("Failed to create temp dir: {}", e),
                rust_source: source_code.to_string(),
            })?;

        let src_file = temp_dir.path().join("main.rs");
        fs::write(&src_file, source_code).map_err(|e| CompilationError {
            stderr: format!("Failed to write source file: {}", e),
            rust_source: source_code.to_string(),
        })?;

        let mut cmd = Command::new("rustc");
        cmd.arg(&src_file)
            .arg("-o")
            .arg(output_bin)
            .arg("-C")
            .arg(format!("opt-level={}", self.opt_level))
            .arg("--edition=2024")
            .stderr(Stdio::piped())
            .stdout(Stdio::piped());

        let output = cmd.output().map_err(|e| CompilationError {
            stderr: format!("Failed to invoke rustc: {}", e),
            rust_source: source_code.to_string(),
        })?;

        if !output.status.success() {
            let stderr_str = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(CompilationError {
                stderr: stderr_str,
                rust_source: source_code.to_string(),
            });
        }

        Ok(())
    }

    /// Compile and run the Rust source code immediately
    pub fn run_source(&self, source_code: &str, args: &[String]) -> Result<ExecutionResult> {
        let temp_dir = Builder::new().prefix("ailang_exec_").tempdir()?;
        let bin_path = temp_dir.path().join("runner_bin");

        let compile_start = Instant::now();
        if let Err(compile_err) = self.compile(source_code, &bin_path) {
            return Err(anyhow!(compile_err));
        }
        let compile_duration = compile_start.elapsed().as_millis();

        let run_start = Instant::now();
        let child = Command::new(&bin_path)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn compiled binary")?;

        let output = child.wait_with_output()?;
        let run_duration = run_start.elapsed().as_millis();

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let preserved_bin = if self.keep_temp {
            let saved_path = std::env::temp_dir().join(format!("ailang_bin_{}", compile_start.elapsed().as_nanos()));
            let _ = fs::copy(&bin_path, &saved_path);
            Some(saved_path)
        } else {
            None
        };

        Ok(ExecutionResult {
            success: output.status.success(),
            stdout,
            stderr,
            exit_code: output.status.code(),
            compile_duration_ms: compile_duration,
            run_duration_ms: run_duration,
            rust_source: source_code.to_string(),
            binary_path: preserved_bin,
        })
    }

    /// Check if rustc is available on the system
    pub fn check_rustc() -> Result<String> {
        let output = Command::new("rustc")
            .arg("--version")
            .output()
            .context("`rustc` was not found in PATH. Please install Rust from https://rustup.rs/")?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err(anyhow!("Failed to run `rustc --version`"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_runner_valid_execution() {
        let runner = RustRunner::new(false);
        let src = r#"fn main() { println!("test_ok_123"); }"#;
        let res = runner.run_source(src, &[]).expect("Should run successfully");
        assert!(res.success);
        assert_eq!(res.stdout.trim(), "test_ok_123");
    }

    #[test]
    fn test_rust_runner_compilation_failure() {
        let runner = RustRunner::new(false);
        let invalid_src = r#"fn main() { this is not rust code }"#;
        let err = runner.run_source(invalid_src, &[]).unwrap_err();
        let comp_err = err.downcast_ref::<CompilationError>().expect("Should be a CompilationError");
        assert!(comp_err.stderr.contains("error"));
    }
}

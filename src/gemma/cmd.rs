use super::GemmaBackend;
use anyhow::{Context, Result, anyhow};
use std::io::Write;
use std::process::{Command, Stdio};

pub struct CommandBackend {
    command_str: String,
}

impl CommandBackend {
    pub fn new(command_str: &str) -> Self {
        Self {
            command_str: command_str.to_string(),
        }
    }
}

impl GemmaBackend for CommandBackend {
    fn name(&self) -> &str {
        "Local Command (CLI)"
    }

    fn translate(&self, prompt: &str, system_prompt: &str) -> Result<String> {
        let full_input = format!(
            "<start_of_turn>user\n{}\n\n{}<end_of_turn>\n<start_of_turn>model\n",
            system_prompt, prompt
        );

        let parts: Vec<&str> = self.command_str.split_whitespace().collect();
        if parts.is_empty() {
            return Err(anyhow!("Empty command specified for command backend"));
        }

        let mut child = Command::new(parts[0])
            .args(&parts[1..])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("Failed to spawn command `{}`", parts[0]))?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(full_input.as_bytes())?;
        }

        let output = child.wait_with_output()?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Gemma command failed: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(stdout)
    }
}

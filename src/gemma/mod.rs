pub mod cmd;
pub mod google;
pub mod mock;
pub mod ollama;
pub mod openai;

use crate::config::{BackendKind, Config};
use anyhow::Result;

pub trait GemmaBackend: Send + Sync {
    fn name(&self) -> &str;
    fn translate(&self, prompt: &str, system_prompt: &str) -> Result<String>;
    fn check_health(&self) -> Result<bool> {
        Ok(true)
    }
}

pub fn create_backend(config: &Config) -> Box<dyn GemmaBackend> {
    match config.backend {
        BackendKind::Ollama => Box::new(ollama::OllamaBackend::new(
            config.endpoint.as_deref().unwrap_or("http://127.0.0.1:11434"),
            &config.model,
            config.timeout_secs,
        )),
        BackendKind::Google => Box::new(google::GoogleBackend::new(
            config.api_key.as_deref().unwrap_or(""),
            &config.model,
            config.timeout_secs,
        )),
        BackendKind::OpenAi => Box::new(openai::OpenAiBackend::new(
            config.endpoint.as_deref().unwrap_or("http://localhost:8000/v1"),
            config.api_key.as_deref(),
            &config.model,
            config.timeout_secs,
        )),
        BackendKind::Command => Box::new(cmd::CommandBackend::new(
            config.command.as_deref().unwrap_or("gemma"),
        )),
        BackendKind::Mock => Box::new(mock::MockBackend::new()),
        BackendKind::Auto => {
            // Check if Ollama is accessible
            let ollama = ollama::OllamaBackend::new(
                config.endpoint.as_deref().unwrap_or("http://127.0.0.1:11434"),
                &config.model,
                2, // short timeout for probe
            );
            if ollama.check_health().unwrap_or(false) {
                return Box::new(ollama);
            }

            // Check if Google API key is set
            if let Some(key) = &config.api_key {
                if !key.trim().is_empty() {
                    return Box::new(google::GoogleBackend::new(
                        key,
                        &config.model,
                        config.timeout_secs,
                    ));
                }
            }

            // Fallback to Mock with notification
            Box::new(mock::MockBackend::new())
        }
    }
}

/// Helper function to clean and extract code from LLM response
pub fn extract_rust_code(response: &str) -> String {
    let trimmed = response.trim();

    // Check for ```rust ... ``` or ``` ... ```
    if let Some(start_idx) = trimmed.find("```rust") {
        let code_part = &trimmed[start_idx + 7..];
        if let Some(end_idx) = code_part.find("```") {
            return code_part[..end_idx].trim().to_string();
        }
    } else if let Some(start_idx) = trimmed.find("```") {
        let code_part = &trimmed[start_idx + 3..];
        if let Some(end_idx) = code_part.find("```") {
            return code_part[..end_idx].trim().to_string();
        }
    }

    // If no markdown fence found, strip any conversational prefix/suffix if present, or return as is
    trimmed.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_rust_code_with_fence() {
        let text = "Here is the code:\n```rust\nfn main() {\n    println!(\"hi\");\n}\n```\nHope that helps!";
        let extracted = extract_rust_code(text);
        assert_eq!(extracted, "fn main() {\n    println!(\"hi\");\n}");
    }

    #[test]
    fn test_extract_rust_code_plain_fence() {
        let text = "```\nlet x = 10;\n```";
        let extracted = extract_rust_code(text);
        assert_eq!(extracted, "let x = 10;");
    }

    #[test]
    fn test_extract_rust_code_raw() {
        let text = "fn main() {}";
        let extracted = extract_rust_code(text);
        assert_eq!(extracted, "fn main() {}");
    }
}

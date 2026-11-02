use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum BackendKind {
    Auto,
    Ollama,
    Google,
    OpenAi,
    Command,
    Mock,
}

impl Default for BackendKind {
    fn default() -> Self {
        BackendKind::Auto
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub backend: BackendKind,
    pub model: String,
    pub api_key: Option<String>,
    pub endpoint: Option<String>,
    pub command: Option<String>,
    pub max_retries: usize,
    pub timeout_secs: u64,
    pub verbose: bool,
    pub show_rust: bool,
    pub keep_temp: bool,
}

impl Default for Config {
    fn default() -> Self {
        let (backend, model, api_key, endpoint) = Self::detect_environment();

        let timeout_secs = std::env::var("AILANG_TIMEOUT")
            .or_else(|_| std::env::var("OLLAMA_TIMEOUT"))
            .ok()
            .and_then(|t| t.parse::<u64>().ok())
            .unwrap_or(90);

        Self {
            backend,
            model,
            api_key,
            endpoint,
            command: std::env::var("GEMMA_CMD").ok(),
            max_retries: 3,
            timeout_secs,
            verbose: false,
            show_rust: false,
            keep_temp: false,
        }
    }
}

impl Config {
    fn detect_environment() -> (BackendKind, String, Option<String>, Option<String>) {
        if let Ok(backend_str) = std::env::var("GEMMA_BACKEND") {
            match backend_str.to_lowercase().as_str() {
                "ollama" => {
                    let ep = std::env::var("OLLAMA_HOST").unwrap_or_else(|_| "http://127.0.0.1:11434".to_string());
                    let model = std::env::var("OLLAMA_MODEL")
                        .or_else(|_| std::env::var("GEMMA_MODEL"))
                        .unwrap_or_else(|_| "qwen2.5-coder:0.5b".to_string());
                    return (BackendKind::Ollama, model, None, Some(ep));
                }
                "google" | "gemini" => {
                    let key = std::env::var("GEMMA_API_KEY")
                        .or_else(|_| std::env::var("GEMINI_API_KEY"))
                        .ok();
                    let model = std::env::var("GEMMA_MODEL").unwrap_or_else(|_| "gemma-2-27b-it".to_string());
                    return (BackendKind::Google, model, key, None);
                }
                "openai" | "vllm" => {
                    let ep = std::env::var("OPENAI_BASE_URL").unwrap_or_else(|_| "http://localhost:8000/v1".to_string());
                    let key = std::env::var("OPENAI_API_KEY").ok();
                    let model = std::env::var("GEMMA_MODEL").unwrap_or_else(|_| "gemma-2-9b-it".to_string());
                    return (BackendKind::OpenAi, model, key, Some(ep));
                }
                "cmd" | "command" => {
                    let model = std::env::var("GEMMA_MODEL").unwrap_or_else(|_| "gemma".to_string());
                    return (BackendKind::Command, model, None, None);
                }
                "mock" => {
                    return (BackendKind::Mock, "gemma-mock".to_string(), None, None);
                }
                _ => {}
            }
        }

        // Auto-detection:
        if let Ok(key) = std::env::var("GEMMA_API_KEY").or_else(|_| std::env::var("GEMINI_API_KEY")) {
            let model = std::env::var("GEMMA_MODEL").unwrap_or_else(|_| "gemma-2-27b-it".to_string());
            return (BackendKind::Google, model, Some(key), None);
        }

        if let Ok(ep) = std::env::var("OLLAMA_HOST") {
            let model = std::env::var("OLLAMA_MODEL")
                .or_else(|_| std::env::var("GEMMA_MODEL"))
                .unwrap_or_else(|_| "qwen2.5-coder:0.5b".to_string());
            return (BackendKind::Ollama, model, None, Some(ep));
        }

        // Default to Auto (which checks Ollama ping, then falls back to Mock with user notice)
        let model = std::env::var("OLLAMA_MODEL")
            .or_else(|_| std::env::var("GEMMA_MODEL"))
            .unwrap_or_else(|_| "qwen2.5-coder:0.5b".to_string());
        (BackendKind::Auto, model, None, Some("http://127.0.0.1:11434".to_string()))
    }

    pub fn config_file_path() -> Option<PathBuf> {
        let home = std::env::var("HOME").ok()?;
        Some(PathBuf::from(home).join(".ailang").join("config.json"))
    }

    pub fn load() -> Self {
        if let Some(path) = Self::config_file_path() {
            if path.exists() {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(cfg) = serde_json::from_str::<Config>(&content) {
                        return cfg;
                    }
                }
            }
        }
        Config::default()
    }

    #[allow(dead_code)]
    pub fn save(&self) -> anyhow::Result<()> {
        if let Some(path) = Self::config_file_path() {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let data = serde_json::to_string_pretty(self)?;
            std::fs::write(path, data)?;
        }
        Ok(())
    }
}

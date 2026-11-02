use super::GemmaBackend;
use anyhow::{Context, Result, anyhow};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub struct OllamaBackend {
    endpoint: String,
    model: String,
    client: Client,
}

#[derive(Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage<'a>>,
    stream: bool,
    options: ChatOptions,
}

#[derive(Serialize)]
struct ChatOptions {
    temperature: f32,
    top_p: f32,
}

#[derive(Deserialize)]
struct ChatResponse {
    message: Option<ResponseMessage>,
    error: Option<String>,
}

#[derive(Deserialize)]
struct ResponseMessage {
    content: String,
}

impl OllamaBackend {
    pub fn new(endpoint: &str, model: &str, timeout_secs: u64) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .unwrap_or_default();

        Self {
            endpoint: endpoint.trim_end_matches('/').to_string(),
            model: model.to_string(),
            client,
        }
    }
}

impl GemmaBackend for OllamaBackend {
    fn name(&self) -> &str {
        "Ollama (Local Gemma)"
    }

    fn check_health(&self) -> Result<bool> {
        let url = format!("{}/api/tags", self.endpoint);
        let resp = self.client.get(&url).send()?;
        Ok(resp.status().is_success())
    }

    fn translate(&self, prompt: &str, system_prompt: &str) -> Result<String> {
        let url = format!("{}/api/chat", self.endpoint);

        let req = ChatRequest {
            model: &self.model,
            messages: vec![
                ChatMessage {
                    role: "system",
                    content: system_prompt,
                },
                ChatMessage {
                    role: "user",
                    content: prompt,
                },
            ],
            stream: false,
            options: ChatOptions {
                temperature: 0.1,
                top_p: 0.9,
            },
        };

        let response = self
            .client
            .post(&url)
            .json(&req)
            .send()
            .with_context(|| format!("Failed to connect to Ollama at {}. Is Ollama running?", self.endpoint))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(anyhow!("Ollama API error {}: {}", status, body));
        }

        let chat_res: ChatResponse = response.json().context("Failed to parse Ollama response")?;

        if let Some(err) = chat_res.error {
            return Err(anyhow!("Ollama returned error: {}", err));
        }

        chat_res
            .message
            .map(|m| m.content)
            .ok_or_else(|| anyhow!("Ollama response was empty"))
    }
}

use super::GemmaBackend;
use anyhow::{Context, Result, anyhow};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub struct OpenAiBackend {
    endpoint: String,
    api_key: Option<String>,
    model: String,
    client: Client,
}

#[derive(Serialize)]
struct OpenAiChatRequest<'a> {
    model: &'a str,
    messages: Vec<OpenAiMessage<'a>>,
    temperature: f32,
}

#[derive(Serialize)]
struct OpenAiMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct OpenAiChatResponse {
    choices: Option<Vec<OpenAiChoice>>,
    error: Option<OpenAiError>,
}

#[derive(Deserialize)]
struct OpenAiChoice {
    message: Option<OpenAiResponseMessage>,
}

#[derive(Deserialize)]
struct OpenAiResponseMessage {
    content: Option<String>,
}

#[derive(Deserialize)]
struct OpenAiError {
    message: String,
}

impl OpenAiBackend {
    pub fn new(endpoint: &str, api_key: Option<&str>, model: &str, timeout_secs: u64) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .unwrap_or_default();

        Self {
            endpoint: endpoint.trim_end_matches('/').to_string(),
            api_key: api_key.map(|s| s.to_string()),
            model: model.to_string(),
            client,
        }
    }
}

impl GemmaBackend for OpenAiBackend {
    fn name(&self) -> &str {
        "OpenAI-Compatible (vLLM / llama.cpp / Groq)"
    }

    fn translate(&self, prompt: &str, system_prompt: &str) -> Result<String> {
        let url = format!("{}/chat/completions", self.endpoint);

        let req = OpenAiChatRequest {
            model: &self.model,
            messages: vec![
                OpenAiMessage {
                    role: "system",
                    content: system_prompt,
                },
                OpenAiMessage {
                    role: "user",
                    content: prompt,
                },
            ],
            temperature: 0.1,
        };

        let mut req_builder = self.client.post(&url).json(&req);

        if let Some(key) = &self.api_key {
            req_builder = req_builder.header("Authorization", format!("Bearer {}", key));
        }

        let response = req_builder
            .send()
            .with_context(|| format!("Failed to connect to endpoint at {}", self.endpoint))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(anyhow!("OpenAI API error {}: {}", status, body));
        }

        let res_data: OpenAiChatResponse = response.json().context("Failed to parse response")?;

        if let Some(err) = res_data.error {
            return Err(anyhow!("API returned error: {}", err.message));
        }

        let content = res_data
            .choices
            .and_then(|c| c.into_iter().next())
            .and_then(|c| c.message)
            .and_then(|m| m.content)
            .ok_or_else(|| anyhow!("API response had no content"))?;

        Ok(content)
    }
}

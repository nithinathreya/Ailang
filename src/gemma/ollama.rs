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
struct GenerateRequest<'a> {
    model: &'a str,
    system: &'a str,
    prompt: &'a str,
    stream: bool,
    options: GenerateOptions,
}

#[derive(Serialize)]
struct GenerateOptions {
    temperature: f32,
    top_p: f32,
}

#[derive(Deserialize)]
struct GenerateResponse {
    response: Option<String>,
    error: Option<String>,
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
        "Ollama (Local LLM)"
    }

    fn check_health(&self) -> Result<bool> {
        let url = format!("{}/api/tags", self.endpoint);
        let resp = self.client.get(&url).send()?;
        Ok(resp.status().is_success())
    }

    fn translate(&self, prompt: &str, system_prompt: &str) -> Result<String> {
        let url = format!("{}/api/generate", self.endpoint);

        let req = GenerateRequest {
            model: &self.model,
            system: system_prompt,
            prompt,
            stream: false,
            options: GenerateOptions {
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

        let gen_res: GenerateResponse = response.json().context("Failed to parse Ollama response")?;

        if let Some(err) = gen_res.error {
            return Err(anyhow!("Ollama returned error: {}", err));
        }

        gen_res
            .response
            .ok_or_else(|| anyhow!("Ollama response was empty"))
    }
}

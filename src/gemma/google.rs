use super::GemmaBackend;
use anyhow::{Context, Result, anyhow};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub struct GoogleBackend {
    api_key: String,
    model: String,
    client: Client,
}

#[derive(Serialize)]
struct GoogleRequest<'a> {
    system_instruction: GoogleSystemInstruction<'a>,
    contents: Vec<GoogleContent<'a>>,
    #[serde(rename = "generationConfig")]
    generation_config: GoogleGenConfig,
}

#[derive(Serialize)]
struct GoogleSystemInstruction<'a> {
    parts: Vec<GooglePart<'a>>,
}

#[derive(Serialize)]
struct GoogleContent<'a> {
    role: &'a str,
    parts: Vec<GooglePart<'a>>,
}

#[derive(Serialize)]
struct GooglePart<'a> {
    text: &'a str,
}

#[derive(Serialize)]
struct GoogleGenConfig {
    temperature: f32,
    #[serde(rename = "topP")]
    top_p: f32,
}

#[derive(Deserialize)]
struct GoogleResponse {
    candidates: Option<Vec<GoogleCandidate>>,
    error: Option<GoogleError>,
}

#[derive(Deserialize)]
struct GoogleCandidate {
    content: Option<GoogleCandidateContent>,
}

#[derive(Deserialize)]
struct GoogleCandidateContent {
    parts: Option<Vec<GoogleResponsePart>>,
}

#[derive(Deserialize)]
struct GoogleResponsePart {
    text: Option<String>,
}

#[derive(Deserialize)]
struct GoogleError {
    message: String,
}

impl GoogleBackend {
    pub fn new(api_key: &str, model: &str, timeout_secs: u64) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .unwrap_or_default();

        Self {
            api_key: api_key.to_string(),
            model: model.to_string(),
            client,
        }
    }
}

impl GemmaBackend for GoogleBackend {
    fn name(&self) -> &str {
        "Google AI Studio (Gemma / Gemini API)"
    }

    fn check_health(&self) -> Result<bool> {
        Ok(!self.api_key.is_empty())
    }

    fn translate(&self, prompt: &str, system_prompt: &str) -> Result<String> {
        if self.api_key.is_empty() {
            return Err(anyhow!(
                "Google API key is missing. Set GEMMA_API_KEY or GEMINI_API_KEY environment variable."
            ));
        }

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, self.api_key
        );

        let req = GoogleRequest {
            system_instruction: GoogleSystemInstruction {
                parts: vec![GooglePart { text: system_prompt }],
            },
            contents: vec![GoogleContent {
                role: "user",
                parts: vec![GooglePart { text: prompt }],
            }],
            generation_config: GoogleGenConfig {
                temperature: 0.1,
                top_p: 0.95,
            },
        };

        let response = self
            .client
            .post(&url)
            .json(&req)
            .send()
            .context("Failed to connect to Google Generative AI API")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(anyhow!("Google API error {}: {}", status, body));
        }

        let res_data: GoogleResponse = response.json().context("Failed to parse Google API response")?;

        if let Some(err) = res_data.error {
            return Err(anyhow!("Google API returned error: {}", err.message));
        }

        let candidate = res_data
            .candidates
            .and_then(|c| c.into_iter().next())
            .and_then(|c| c.content)
            .and_then(|content| content.parts)
            .and_then(|parts| parts.into_iter().next())
            .and_then(|part| part.text)
            .ok_or_else(|| anyhow!("Google API returned no text content"))?;

        Ok(candidate)
    }
}

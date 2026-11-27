//! Model endpoints. Everything is plain HTTP so the benchmark needs no native ML
//! libraries: run local models with llama.cpp's `llama-server` (OpenAI-compatible,
//! `--mmproj` for vision), use OpenRouter, or the Anthropic Messages API.

use anyhow::{Context, Result, bail};
use base64::Engine;
use serde::Deserialize;
use serde_json::{Value, json};
use std::time::Duration;

#[derive(Debug, Clone, Deserialize)]
pub struct ModelConfig {
    /// Label used in reports.
    pub name: String,
    /// `openai` (llama-server, OpenRouter, vLLM …) or `anthropic`.
    pub api: Api,
    /// For `openai`: e.g. `http://127.0.0.1:8080/v1` or `https://openrouter.ai/api/v1`.
    #[serde(default)]
    pub base_url: Option<String>,
    /// Model id sent to the endpoint.
    pub model: String,
    /// Environment variable holding the API key (never put keys in the file).
    #[serde(default)]
    pub api_key_env: Option<String>,
    /// Which tasks to run: `text`, `math`.
    #[serde(default = "default_tasks")]
    pub tasks: Vec<String>,
    /// Anthropic only: `low` | `medium` | `high` …
    #[serde(default)]
    pub effort: Option<String>,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
}

fn default_tasks() -> Vec<String> {
    vec!["text".into(), "math".into()]
}

fn default_max_tokens() -> u32 {
    4096
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Api {
    Openai,
    Anthropic,
}

pub struct Client {
    http: reqwest::blocking::Client,
}

pub struct Reply {
    pub text: String,
    pub stop_reason: Option<String>,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
}

impl Client {
    pub fn new() -> Result<Self> {
        let http = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(900))
            .user_agent("pennoa-bench")
            .build()?;
        Ok(Self { http })
    }

    pub fn http(&self) -> &reqwest::blocking::Client {
        &self.http
    }

    /// Send one image + instruction, return the model's text.
    pub fn vision(
        &self,
        m: &ModelConfig,
        image: &[u8],
        media_type: &str,
        prompt: &str,
    ) -> Result<Reply> {
        let b64 = base64::engine::general_purpose::STANDARD.encode(image);
        let key = match &m.api_key_env {
            Some(var) if !var.is_empty() => Some(
                std::env::var(var)
                    .with_context(|| format!("environment variable {var} is not set"))?,
            ),
            _ => None,
        };
        match m.api {
            Api::Openai => {
                let base = m
                    .base_url
                    .as_deref()
                    .unwrap_or("http://127.0.0.1:8080/v1")
                    .trim_end_matches('/');
                let body = json!({
                    "model": m.model,
                    "max_tokens": m.max_tokens,
                    "temperature": 0,
                    "messages": [{
                        "role": "user",
                        "content": [
                            {"type": "image_url", "image_url": {"url": format!("data:{media_type};base64,{b64}")}},
                            {"type": "text", "text": prompt}
                        ]
                    }]
                });
                let mut req = self
                    .http
                    .post(format!("{base}/chat/completions"))
                    .json(&body);
                if let Some(k) = key {
                    req = req.bearer_auth(k);
                }
                let resp = req.send()?;
                let status = resp.status();
                let v: Value = resp.json().context("response was not JSON")?;
                if !status.is_success() {
                    bail!("{status}: {v}");
                }
                let choice = &v["choices"][0];
                Ok(Reply {
                    text: choice["message"]["content"]
                        .as_str()
                        .unwrap_or_default()
                        .to_string(),
                    stop_reason: choice["finish_reason"].as_str().map(str::to_string),
                    input_tokens: v["usage"]["prompt_tokens"].as_u64(),
                    output_tokens: v["usage"]["completion_tokens"].as_u64(),
                })
            }
            Api::Anthropic => {
                let key =
                    key.context("anthropic models need api_key_env (e.g. ANTHROPIC_API_KEY)")?;
                let base = m
                    .base_url
                    .as_deref()
                    .unwrap_or("https://api.anthropic.com")
                    .trim_end_matches('/');
                let mut body = json!({
                    "model": m.model,
                    "max_tokens": m.max_tokens,
                    "messages": [{
                        "role": "user",
                        "content": [
                            {"type": "image", "source": {"type": "base64", "media_type": media_type, "data": b64}},
                            {"type": "text", "text": prompt}
                        ]
                    }]
                });
                if let Some(effort) = &m.effort {
                    body["output_config"] = json!({ "effort": effort });
                }
                let resp = self
                    .http
                    .post(format!("{base}/v1/messages"))
                    .header("x-api-key", key)
                    .header("anthropic-version", "2023-06-01")
                    .json(&body)
                    .send()?;
                let status = resp.status();
                let v: Value = resp.json().context("response was not JSON")?;
                if !status.is_success() {
                    bail!("{status}: {v}");
                }
                // Thinking blocks may precede the answer; keep only text blocks.
                let text = v["content"]
                    .as_array()
                    .map(|blocks| {
                        blocks
                            .iter()
                            .filter(|b| b["type"] == "text")
                            .filter_map(|b| b["text"].as_str())
                            .collect::<Vec<_>>()
                            .join("")
                    })
                    .unwrap_or_default();
                Ok(Reply {
                    text,
                    stop_reason: v["stop_reason"].as_str().map(str::to_string),
                    input_tokens: v["usage"]["input_tokens"].as_u64(),
                    output_tokens: v["usage"]["output_tokens"].as_u64(),
                })
            }
        }
    }
}

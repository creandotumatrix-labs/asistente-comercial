//! Thin Anthropic Messages API client with tool-use support. Messages are kept
//! as raw JSON values so they round-trip cleanly through Postgres (replay/audit)
//! without a lossy intermediate type.

use anyhow::{anyhow, Result};
use serde_json::{json, Value};

use crate::config::AnthropicConfig;

const ENDPOINT: &str = "https://api.anthropic.com/v1/messages";

#[derive(Clone)]
pub struct LlmClient {
    http: reqwest::Client,
    api_key: String,
    model: String,
    version: String,
    max_tokens: u32,
}

#[derive(Debug, Clone)]
pub struct ToolUse {
    pub id: String,
    pub name: String,
    pub input: Value,
}

#[derive(Debug, Clone)]
pub struct LlmResponse {
    pub stop_reason: String,
    /// `{ "role": "assistant", "content": [...] }` — append verbatim to history.
    pub assistant_message: Value,
    /// Concatenated text blocks (the user-facing reply when there are no tools).
    pub text: String,
    pub tool_uses: Vec<ToolUse>,
}

impl LlmClient {
    pub fn new(http: reqwest::Client, cfg: &AnthropicConfig) -> Self {
        Self {
            http,
            api_key: cfg.api_key.clone(),
            model: cfg.model.clone(),
            version: cfg.version.clone(),
            max_tokens: cfg.max_tokens,
        }
    }

    pub async fn create(
        &self,
        system: &str,
        messages: &[Value],
        tools: &[Value],
    ) -> Result<LlmResponse> {
        let body = json!({
            "model": self.model,
            "max_tokens": self.max_tokens,
            "system": system,
            "messages": messages,
            "tools": tools,
        });

        let resp = self
            .http
            .post(ENDPOINT)
            .header("x-api-key", self.api_key.as_str())
            .header("anthropic-version", self.version.as_str())
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let v: Value = resp.json().await.unwrap_or(Value::Null);
        if !status.is_success() {
            return Err(anyhow!("anthropic error ({status}): {v}"));
        }

        let content = v
            .get("content")
            .and_then(|c| c.as_array())
            .cloned()
            .unwrap_or_default();

        let mut text = String::new();
        let mut tool_uses = Vec::new();
        for block in &content {
            match block.get("type").and_then(|t| t.as_str()) {
                Some("text") => {
                    if let Some(t) = block.get("text").and_then(|x| x.as_str()) {
                        text.push_str(t);
                    }
                }
                Some("tool_use") => tool_uses.push(ToolUse {
                    id: block
                        .get("id")
                        .and_then(|x| x.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    name: block
                        .get("name")
                        .and_then(|x| x.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    input: block.get("input").cloned().unwrap_or_else(|| json!({})),
                }),
                _ => {}
            }
        }

        let stop_reason = v
            .get("stop_reason")
            .and_then(|x| x.as_str())
            .unwrap_or("end_turn")
            .to_string();

        Ok(LlmResponse {
            stop_reason,
            assistant_message: json!({ "role": "assistant", "content": content }),
            text,
            tool_uses,
        })
    }
}

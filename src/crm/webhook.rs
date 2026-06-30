//! Generic webhook CRM: POSTs the qualified lead as JSON to any endpoint
//! (Zapier/Make/n8n/your own API). The escape hatch for CRMs we don't natively
//! integrate.

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::json;

use super::{Crm, LeadInput, LeadResult};

pub struct WebhookCrm {
    http: reqwest::Client,
    url: String,
}

impl WebhookCrm {
    pub fn new(http: reqwest::Client, url: String) -> Self {
        Self { http, url }
    }
}

#[async_trait]
impl Crm for WebhookCrm {
    async fn create_lead(&self, input: &LeadInput) -> Result<LeadResult> {
        let body = json!({
            "name": input.name,
            "email": input.email,
            "phone": input.phone,
            "score": input.score,
            "reasons": input.reasons,
            "qualified_fields": input.qualified_fields,
            "summary": input.summary,
        });
        let resp = self.http.post(&self.url).json(&body).send().await?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("crm webhook failed ({status}): {text}"));
        }
        let crm_id = resp
            .json::<serde_json::Value>()
            .await
            .ok()
            .and_then(|v| v.get("id").and_then(|x| x.as_str()).map(str::to_string));
        Ok(LeadResult { crm_id })
    }
}

//! Outbound WhatsApp Cloud API client. We only send free-form text, which is
//! allowed inside the 24h customer-service window opened by an inbound message
//! — exactly the click-to-WhatsApp lead-gen case. (Outside that window Meta
//! requires approved message templates; see README "Reminders".)

use anyhow::{anyhow, Result};
use serde_json::json;

use crate::config::WhatsappConfig;

#[derive(Clone)]
pub struct WhatsappClient {
    http: reqwest::Client,
    phone_number_id: String,
    access_token: String,
    graph_version: String,
}

impl WhatsappClient {
    pub fn new(http: reqwest::Client, cfg: &WhatsappConfig) -> Self {
        Self {
            http,
            phone_number_id: cfg.phone_number_id.clone(),
            access_token: cfg.access_token.clone(),
            graph_version: cfg.graph_version.clone(),
        }
    }

    pub async fn send_text(&self, to: &str, body: &str) -> Result<()> {
        let url = format!(
            "https://graph.facebook.com/{}/{}/messages",
            self.graph_version, self.phone_number_id
        );
        // WhatsApp hard-limits text bodies to 4096 chars.
        let clipped: String = body.chars().take(4096).collect();
        let payload = json!({
            "messaging_product": "whatsapp",
            "recipient_type": "individual",
            "to": to,
            "type": "text",
            "text": { "preview_url": false, "body": clipped }
        });

        let resp = self
            .http
            .post(&url)
            .bearer_auth(&self.access_token)
            .json(&payload)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("WhatsApp send failed ({status}): {text}"));
        }
        Ok(())
    }
}

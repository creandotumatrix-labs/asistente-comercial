//! Rep handoff. A qualified lead is pushed to the human rep with the full
//! decision context: score + reasons + answers + contact + transcript link.

use anyhow::Result;
use serde::Serialize;

use crate::config::Handoff;
use crate::whatsapp::WhatsappClient;

#[derive(Debug, Clone, Serialize)]
pub struct HandoffPayload {
    pub score: String,
    pub name: Option<String>,
    pub contact: Option<String>,
    pub reasons: Vec<String>,
    pub summary: String,
    pub transcript_url: String,
    pub meeting_label: Option<String>,
}

impl HandoffPayload {
    fn render_text(&self) -> String {
        let mut s = format!("🔔 Nuevo lead *{}*\n", self.score.to_uppercase());
        s.push_str(&format!(
            "👤 {}\n",
            self.name.as_deref().unwrap_or("(sin nombre)")
        ));
        if let Some(c) = &self.contact {
            s.push_str(&format!("📞 {c}\n"));
        }
        if let Some(m) = &self.meeting_label {
            s.push_str(&format!("📅 {m}\n"));
        }
        if !self.summary.is_empty() {
            s.push_str(&format!("📝 {}\n", self.summary));
        }
        if !self.reasons.is_empty() {
            s.push_str("\nPor qué:\n");
            for r in &self.reasons {
                s.push_str(&format!("• {r}\n"));
            }
        }
        s.push_str(&format!("\n🧾 Transcripción: {}", self.transcript_url));
        s
    }
}

#[derive(Clone)]
pub struct Notifier {
    whatsapp: WhatsappClient,
    http: reqwest::Client,
}

impl Notifier {
    pub fn new(whatsapp: WhatsappClient, http: reqwest::Client) -> Self {
        Self { whatsapp, http }
    }

    pub async fn handoff(&self, cfg: &Handoff, payload: &HandoffPayload) -> Result<()> {
        match cfg.channel.as_str() {
            "whatsapp" => match &cfg.rep_wa_id {
                Some(rep) => self.whatsapp.send_text(rep, &payload.render_text()).await?,
                None => tracing::warn!("handoff channel=whatsapp but rep_wa_id is unset"),
            },
            "webhook" => match &cfg.webhook_url {
                Some(url) => {
                    let resp = self.http.post(url).json(payload).send().await?;
                    if !resp.status().is_success() {
                        tracing::warn!(status = %resp.status(), "handoff webhook non-2xx");
                    }
                }
                None => tracing::warn!("handoff channel=webhook but webhook_url is unset"),
            },
            "none" => {}
            other => tracing::warn!("unknown handoff channel `{other}`"),
        }
        Ok(())
    }
}

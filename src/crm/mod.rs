//! CRM abstraction. The agent routes a qualified lead through `create_lead`;
//! the concrete backend (HubSpot or a generic webhook) is chosen by config.

pub mod hubspot;
pub mod webhook;

pub use hubspot::HubSpotCrm;
pub use webhook::WebhookCrm;

use std::sync::Arc;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::Value;

use crate::config::{CrmBackend, CrmConfig};

#[derive(Debug, Clone)]
pub struct LeadInput {
    pub name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    /// "hot" | "warm" | "cold"
    pub score: String,
    pub reasons: Vec<String>,
    /// The extracted, qualified slot values.
    pub qualified_fields: Value,
    pub summary: String,
}

#[derive(Debug, Clone)]
pub struct LeadResult {
    pub crm_id: Option<String>,
}

#[async_trait]
pub trait Crm: Send + Sync {
    async fn create_lead(&self, input: &LeadInput) -> Result<LeadResult>;
}

/// Construct the configured CRM backend.
pub fn build(http: reqwest::Client, cfg: &CrmConfig) -> Result<Arc<dyn Crm>> {
    match cfg.backend {
        CrmBackend::HubSpot => {
            let token = cfg
                .hubspot_token
                .clone()
                .ok_or_else(|| anyhow!("HUBSPOT_TOKEN required for CRM_BACKEND=hubspot"))?;
            Ok(Arc::new(HubSpotCrm::new(
                http,
                token,
                cfg.hubspot_pipeline.clone(),
                cfg.hubspot_dealstage.clone(),
            )))
        }
        CrmBackend::Webhook => {
            let url = cfg
                .webhook_url
                .clone()
                .ok_or_else(|| anyhow!("CRM_WEBHOOK_URL required for CRM_BACKEND=webhook"))?;
            Ok(Arc::new(WebhookCrm::new(http, url)))
        }
    }
}

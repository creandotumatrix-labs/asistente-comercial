//! HubSpot CRM v3. Upserts a contact, creates a deal carrying the lead score +
//! reasons, and associates them. Resilient to the "contact already exists"
//! conflict (parses the existing id and proceeds).

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use reqwest::StatusCode;
use serde_json::{json, Map, Value};

use super::{Crm, LeadInput, LeadResult};

const CONTACTS_URL: &str = "https://api.hubapi.com/crm/v3/objects/contacts";
const DEALS_URL: &str = "https://api.hubapi.com/crm/v3/objects/deals";
/// HubSpot-defined association type id for deal → contact.
const DEAL_TO_CONTACT: i64 = 3;

pub struct HubSpotCrm {
    http: reqwest::Client,
    token: String,
    pipeline: String,
    dealstage: String,
}

impl HubSpotCrm {
    pub fn new(http: reqwest::Client, token: String, pipeline: String, dealstage: String) -> Self {
        Self {
            http,
            token,
            pipeline,
            dealstage,
        }
    }

    async fn post(&self, url: &str, body: &Value) -> Result<(StatusCode, Value)> {
        let resp = self
            .http
            .post(url)
            .bearer_auth(&self.token)
            .json(body)
            .send()
            .await?;
        let status = resp.status();
        let v = resp.json::<Value>().await.unwrap_or(Value::Null);
        Ok((status, v))
    }

    async fn upsert_contact(&self, input: &LeadInput) -> Result<Option<String>> {
        let (first, last) = split_name(input.name.as_deref());
        let mut props = Map::new();
        if let Some(e) = &input.email {
            props.insert("email".into(), json!(e));
        }
        if let Some(f) = first {
            props.insert("firstname".into(), json!(f));
        }
        if let Some(l) = last {
            props.insert("lastname".into(), json!(l));
        }
        if let Some(p) = &input.phone {
            props.insert("phone".into(), json!(p));
        }
        props.insert("hs_lead_status".into(), json!(lead_status(&input.score)));

        let (status, body) = self.post(CONTACTS_URL, &json!({ "properties": props })).await?;
        if status.is_success() {
            return Ok(body.get("id").and_then(|x| x.as_str()).map(str::to_string));
        }
        if status == StatusCode::CONFLICT {
            // "Contact already exists. Existing ID: 12345"
            if let Some(id) = parse_existing_id(&body) {
                return Ok(Some(id));
            }
        }
        Err(anyhow!("hubspot contact upsert failed ({status}): {body}"))
    }

    async fn create_deal(&self, input: &LeadInput, contact_id: Option<&str>) -> Result<Option<String>> {
        let mut props = Map::new();
        props.insert("dealname".into(), json!(deal_name(input)));
        props.insert("pipeline".into(), json!(self.pipeline));
        props.insert("dealstage".into(), json!(self.dealstage));
        props.insert("description".into(), json!(deal_description(input)));

        let mut body = json!({ "properties": props });
        if let Some(cid) = contact_id {
            body["associations"] = json!([{
                "to": { "id": cid },
                "types": [{
                    "associationCategory": "HUBSPOT_DEFINED",
                    "associationTypeId": DEAL_TO_CONTACT
                }]
            }]);
        }

        let (status, resp) = self.post(DEALS_URL, &body).await?;
        if !status.is_success() {
            return Err(anyhow!("hubspot deal create failed ({status}): {resp}"));
        }
        Ok(resp.get("id").and_then(|x| x.as_str()).map(str::to_string))
    }
}

#[async_trait]
impl Crm for HubSpotCrm {
    async fn create_lead(&self, input: &LeadInput) -> Result<LeadResult> {
        let contact_id = self.upsert_contact(input).await?;
        let deal_id = self.create_deal(input, contact_id.as_deref()).await?;
        Ok(LeadResult {
            crm_id: deal_id.or(contact_id),
        })
    }
}

fn lead_status(score: &str) -> &'static str {
    match score {
        "hot" => "OPEN_DEAL",
        "warm" => "OPEN",
        _ => "UNQUALIFIED",
    }
}

fn deal_name(input: &LeadInput) -> String {
    let who = input.name.as_deref().unwrap_or("Prospecto");
    format!("{who} — lead {} (WhatsApp)", input.score)
}

fn deal_description(input: &LeadInput) -> String {
    let mut s = input.summary.clone();
    if !input.reasons.is_empty() {
        s.push_str("\n\nRazones del puntaje:\n");
        for r in &input.reasons {
            s.push_str("• ");
            s.push_str(r);
            s.push('\n');
        }
    }
    s.push_str("\nDatos calificados: ");
    s.push_str(&input.qualified_fields.to_string());
    s
}

fn split_name(name: Option<&str>) -> (Option<String>, Option<String>) {
    match name {
        Some(n) if !n.trim().is_empty() => {
            let n = n.trim();
            let mut parts = n.splitn(2, ' ');
            let first = parts.next().map(str::to_string);
            let last = parts.next().map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
            (first, last)
        }
        _ => (None, None),
    }
}

fn parse_existing_id(body: &Value) -> Option<String> {
    let msg = body.get("message").and_then(|m| m.as_str())?;
    let idx = msg.find("Existing ID:")?;
    let tail = &msg[idx + "Existing ID:".len()..];
    let id: String = tail
        .trim()
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    (!id.is_empty()).then_some(id)
}

//! Configuration: process-level `AppConfig` (from environment) and the
//! white-label `Offer` (from `offer.json`). The Offer is the entire
//! "swap → new business in minutes" surface: branding, qualification slots,
//! scoring rubric, disqualifiers, calendar target, resources, handoff.

use std::collections::BTreeMap;
use std::env;

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;

// ─────────────────────────────────────────────────────────────────────────────
// Process configuration (secrets + endpoints, from env)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub bind_addr: String,
    pub offer_config_path: String,
    pub public_base_url: String,
    pub database_url: String,
    pub run_migrations: bool,
    pub anthropic: AnthropicConfig,
    pub whatsapp: WhatsappConfig,
    pub google: GoogleConfig,
    pub crm: CrmConfig,
}

#[derive(Debug, Clone)]
pub struct AnthropicConfig {
    pub api_key: String,
    pub model: String,
    pub version: String,
    pub max_tokens: u32,
}

#[derive(Debug, Clone)]
pub struct WhatsappConfig {
    pub phone_number_id: String,
    pub access_token: String,
    pub graph_version: String,
    pub verify_token: String,
    /// Meta app secret. When set, inbound webhooks must carry a valid
    /// `X-Hub-Signature-256`. When unset, signature checking is skipped.
    pub app_secret: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GoogleConfig {
    /// Raw service-account JSON (already resolved from inline env or file).
    pub service_account_json: Option<String>,
}

#[derive(Debug, Clone)]
pub enum CrmBackend {
    HubSpot,
    Webhook,
}

#[derive(Debug, Clone)]
pub struct CrmConfig {
    pub backend: CrmBackend,
    pub hubspot_token: Option<String>,
    pub hubspot_pipeline: String,
    pub hubspot_dealstage: String,
    pub webhook_url: Option<String>,
}

impl AppConfig {
    pub fn from_env() -> Result<Self> {
        let crm_backend = match env_or("CRM_BACKEND", "hubspot").to_lowercase().as_str() {
            "hubspot" => CrmBackend::HubSpot,
            "webhook" => CrmBackend::Webhook,
            other => return Err(anyhow!("unknown CRM_BACKEND `{other}` (use hubspot|webhook)")),
        };

        let google_service_account_json = match env_opt("GOOGLE_SERVICE_ACCOUNT_JSON") {
            Some(inline) if !inline.trim().is_empty() => Some(inline),
            _ => match env_opt("GOOGLE_SERVICE_ACCOUNT_PATH") {
                Some(path) if !path.trim().is_empty() => Some(
                    std::fs::read_to_string(&path)
                        .with_context(|| format!("reading GOOGLE_SERVICE_ACCOUNT_PATH `{path}`"))?,
                ),
                _ => None,
            },
        };

        Ok(Self {
            bind_addr: env_or("BIND_ADDR", "0.0.0.0:8080"),
            offer_config_path: env_or("OFFER_CONFIG_PATH", "config/offer.json"),
            public_base_url: env_or("PUBLIC_BASE_URL", "http://localhost:8080"),
            database_url: env_req("DATABASE_URL")?,
            run_migrations: env_or("RUN_MIGRATIONS", "true").parse().unwrap_or(true),
            anthropic: AnthropicConfig {
                api_key: env_req("ANTHROPIC_API_KEY")?,
                model: env_or("ANTHROPIC_MODEL", "claude-sonnet-4-6"),
                version: env_or("ANTHROPIC_VERSION", "2023-06-01"),
                max_tokens: env_or("ANTHROPIC_MAX_TOKENS", "1024").parse().unwrap_or(1024),
            },
            whatsapp: WhatsappConfig {
                phone_number_id: env_req("WHATSAPP_PHONE_NUMBER_ID")?,
                access_token: env_req("WHATSAPP_ACCESS_TOKEN")?,
                graph_version: env_or("WHATSAPP_GRAPH_VERSION", "v21.0"),
                verify_token: env_req("WHATSAPP_VERIFY_TOKEN")?,
                app_secret: env_opt("WHATSAPP_APP_SECRET"),
            },
            google: GoogleConfig {
                service_account_json: google_service_account_json,
            },
            crm: CrmConfig {
                backend: crm_backend,
                hubspot_token: env_opt("HUBSPOT_TOKEN"),
                hubspot_pipeline: env_or("HUBSPOT_PIPELINE", "default"),
                hubspot_dealstage: env_or("HUBSPOT_DEALSTAGE", "appointmentscheduled"),
                webhook_url: env_opt("CRM_WEBHOOK_URL"),
            },
        })
    }
}

fn env_req(key: &str) -> Result<String> {
    env::var(key).map_err(|_| anyhow!("missing required env var `{key}`"))
}
fn env_opt(key: &str) -> Option<String> {
    match env::var(key) {
        Ok(v) if !v.is_empty() => Some(v),
        _ => None,
    }
}
fn env_or(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}

// ─────────────────────────────────────────────────────────────────────────────
// White-label Offer (from offer.json) — the entire reconfiguration surface
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct Offer {
    pub branding: Branding,
    pub offer: OfferInfo,
    pub qualification: Qualification,
    pub calendar: CalendarCfg,
    #[serde(default)]
    pub resources: BTreeMap<String, String>,
    pub deflection: Deflection,
    pub handoff: Handoff,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Branding {
    pub business_name: String,
    pub agent_name: String,
    #[serde(default = "default_locale")]
    pub locale: String,
    #[serde(default)]
    pub tone: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OfferInfo {
    pub summary: String,
    #[serde(default)]
    pub what_we_sell: String,
    #[serde(default)]
    pub faq: Vec<Faq>,
    /// Free-text policy the agent must honour, e.g. "defer pricing to the rep".
    #[serde(default)]
    pub pricing_policy: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Faq {
    pub q: String,
    pub a: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Qualification {
    pub slots: Vec<Slot>,
    pub rubric: Rubric,
    #[serde(default)]
    pub disqualifiers: Vec<Disqualifier>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Slot {
    pub key: String,
    pub question: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default, rename = "type")]
    pub kind: SlotKind,
    #[serde(default)]
    pub values: Vec<String>,
    #[serde(default)]
    pub hint: String,
}

#[derive(Debug, Clone, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SlotKind {
    #[default]
    Text,
    Number,
    Enum,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Rubric {
    /// Factor name -> weight. Factors: "size", "timeline", "need_fit"
    /// (and any custom factor your dispatch computes). Need not sum to 1.0.
    pub weights: BTreeMap<String, f64>,
    /// Enum-value -> [0,1] score for the `timeline` factor.
    #[serde(default)]
    pub timeline_scores: BTreeMap<String, f64>,
    /// Numeric size tiers for the `size` factor: size >= min ⇒ score
    /// (highest satisfied `min` wins).
    #[serde(default)]
    pub size_tiers: Vec<SizeTier>,
    /// Keyword -> [0,1] contribution for the `need_fit` factor (max match wins).
    #[serde(default)]
    pub need_keywords: BTreeMap<String, f64>,
    pub thresholds: Thresholds,
    /// Score used for a factor when its slot is missing/unparseable.
    #[serde(default = "default_unknown")]
    pub default_factor_score: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SizeTier {
    pub min: f64,
    pub score: f64,
    #[serde(default)]
    pub label: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Thresholds {
    pub hot: f64,
    pub warm: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Disqualifier {
    pub slot: String,
    #[serde(default)]
    pub contains_any: Vec<String>,
    #[serde(default)]
    pub equals: Option<String>,
    pub reason: String,
    /// Soft = nudge toward cold but don't hard-fail; hard = force cold.
    #[serde(default)]
    pub soft: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CalendarCfg {
    pub calendar_id: String,
    /// IANA tz, e.g. "America/Mexico_City".
    pub timezone: String,
    pub working_hours: WorkingHours,
    #[serde(default = "default_slot_minutes")]
    pub slot_minutes: i64,
    #[serde(default = "default_days_ahead")]
    pub days_ahead: i64,
    /// ISO weekdays that are bookable: 1=Mon … 7=Sun.
    #[serde(default = "default_days")]
    pub days: Vec<u32>,
    /// Don't offer slots starting sooner than now + this many minutes.
    #[serde(default = "default_lead_minutes")]
    pub min_lead_minutes: i64,
    /// Cap the number of candidate slots returned to the agent.
    #[serde(default = "default_max_slots")]
    pub max_slots: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorkingHours {
    /// "HH:MM" 24h, local to `timezone`.
    pub start: String,
    pub end: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Deflection {
    pub message: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Handoff {
    /// "whatsapp" | "webhook" | "none"
    #[serde(default = "default_handoff_channel")]
    pub channel: String,
    /// Rep's WhatsApp wa_id (E.164 without +) for channel=whatsapp.
    #[serde(default)]
    pub rep_wa_id: Option<String>,
    /// Webhook URL for channel=webhook.
    #[serde(default)]
    pub webhook_url: Option<String>,
}

impl Offer {
    pub fn load(path: &str) -> Result<Self> {
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("reading offer config `{path}`"))?;
        let offer: Offer =
            serde_json::from_str(&raw).with_context(|| format!("parsing offer config `{path}`"))?;
        offer.validate()?;
        Ok(offer)
    }

    fn validate(&self) -> Result<()> {
        if self.qualification.slots.is_empty() {
            return Err(anyhow!("offer.qualification.slots must not be empty"));
        }
        let t = &self.qualification.rubric.thresholds;
        if !(0.0..=1.0).contains(&t.hot) || !(0.0..=1.0).contains(&t.warm) || t.warm > t.hot {
            return Err(anyhow!(
                "invalid thresholds: require 0<=warm<=hot<=1 (got warm={}, hot={})",
                t.warm,
                t.hot
            ));
        }
        Ok(())
    }

    /// Required slot keys, in declaration order.
    pub fn required_slot_keys(&self) -> Vec<String> {
        self.qualification
            .slots
            .iter()
            .filter(|s| s.required)
            .map(|s| s.key.clone())
            .collect()
    }
}

fn default_locale() -> String {
    "es-MX".to_string()
}
fn default_unknown() -> f64 {
    0.3
}
fn default_slot_minutes() -> i64 {
    20
}
fn default_days_ahead() -> i64 {
    7
}
fn default_days() -> Vec<u32> {
    vec![1, 2, 3, 4, 5]
}
fn default_lead_minutes() -> i64 {
    120
}
fn default_max_slots() -> usize {
    6
}
fn default_handoff_channel() -> String {
    "whatsapp".to_string()
}

use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct Conversation {
    pub id: Uuid,
    pub wa_id: String,
    pub profile_name: Option<String>,
    pub status: String,
    pub score: Option<String>,
    /// Raw Anthropic message array (JSONB). Source of truth for replay + transcript.
    pub history: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Conversation {
    /// The conversation history as a Vec of message objects (empty if malformed).
    pub fn messages(&self) -> Vec<Value> {
        self.history.as_array().cloned().unwrap_or_default()
    }
}

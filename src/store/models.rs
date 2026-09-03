use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct Conversation {
    pub id: Uuid,
    // Populated by `FromRow` for schema parity with the `conversations` table;
    // not read directly (callers already have the wa_id / timestamps they need
    // from the request or from `id`).
    #[allow(dead_code)]
    pub wa_id: String,
    pub profile_name: Option<String>,
    pub status: String,
    pub score: Option<String>,
    /// Raw Anthropic message array (JSONB). Source of truth for replay + transcript.
    pub history: Value,
    #[allow(dead_code)]
    pub created_at: DateTime<Utc>,
    #[allow(dead_code)]
    pub updated_at: DateTime<Utc>,
}

impl Conversation {
    /// The conversation history as a Vec of message objects (empty if malformed).
    pub fn messages(&self) -> Vec<Value> {
        self.history.as_array().cloned().unwrap_or_default()
    }
}

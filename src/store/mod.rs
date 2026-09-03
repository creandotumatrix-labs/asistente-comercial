//! Postgres persistence. Uses the sqlx runtime query API (no compile-time DB
//! introspection) so the crate builds without a live database.

pub mod models;
pub use models::Conversation;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Executor, PgPool};
use uuid::Uuid;

const MIGRATION: &str = include_str!("../../migrations/0001_init.sql");

#[derive(Clone)]
pub struct Store {
    pool: PgPool,
}

impl Store {
    pub async fn connect(database_url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await?;
        Ok(Self { pool })
    }

    /// Apply the idempotent schema. Safe to call on every boot.
    pub async fn migrate(&self) -> Result<()> {
        (&self.pool).execute(sqlx::raw_sql(MIGRATION)).await?;
        Ok(())
    }

    /// Escape hatch for a caller that needs the raw pool (e.g. a future
    /// admin/metrics endpoint); not called from anywhere yet.
    #[allow(dead_code)]
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Fetch the conversation for a WhatsApp id, creating it on first contact.
    pub async fn get_or_create_conversation(
        &self,
        wa_id: &str,
        profile_name: Option<&str>,
    ) -> Result<Conversation> {
        let id = Uuid::new_v4();
        let conv = sqlx::query_as::<_, Conversation>(
            r#"
            INSERT INTO conversations (id, wa_id, profile_name)
            VALUES ($1, $2, $3)
            ON CONFLICT (wa_id) DO UPDATE
                SET profile_name = COALESCE(EXCLUDED.profile_name, conversations.profile_name),
                    updated_at = now()
            RETURNING id, wa_id, profile_name, status, score, history, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(wa_id)
        .bind(profile_name)
        .fetch_one(&self.pool)
        .await?;
        Ok(conv)
    }

    pub async fn get_conversation(&self, id: Uuid) -> Result<Option<Conversation>> {
        let conv = sqlx::query_as::<_, Conversation>(
            r#"SELECT id, wa_id, profile_name, status, score, history, created_at, updated_at
               FROM conversations WHERE id = $1"#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(conv)
    }

    /// Persist the full message history + derived status/score atomically.
    pub async fn save_conversation(
        &self,
        id: Uuid,
        history: &Value,
        status: &str,
        score: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            r#"UPDATE conversations
               SET history = $2::jsonb, status = $3, score = $4, updated_at = now()
               WHERE id = $1"#,
        )
        .bind(id)
        .bind(history.clone())
        .bind(status)
        .bind(score)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Returns true if this message id was newly recorded (i.e. NOT a redelivery).
    pub async fn mark_message_processed(&self, message_id: &str) -> Result<bool> {
        let res = sqlx::query(
            r#"INSERT INTO processed_messages (message_id) VALUES ($1)
               ON CONFLICT (message_id) DO NOTHING"#,
        )
        .bind(message_id)
        .execute(&self.pool)
        .await?;
        Ok(res.rows_affected() == 1)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn insert_lead(
        &self,
        conversation_id: Uuid,
        name: Option<&str>,
        email: Option<&str>,
        phone: Option<&str>,
        score: &str,
        reasons: &Value,
        fields: &Value,
        crm_id: Option<&str>,
    ) -> Result<Uuid> {
        let id = Uuid::new_v4();
        sqlx::query(
            r#"INSERT INTO leads
               (id, conversation_id, name, email, phone, score, reasons, fields, crm_id)
               VALUES ($1, $2, $3, $4, $5, $6, $7::jsonb, $8::jsonb, $9)"#,
        )
        .bind(id)
        .bind(conversation_id)
        .bind(name)
        .bind(email)
        .bind(phone)
        .bind(score)
        .bind(reasons.clone())
        .bind(fields.clone())
        .bind(crm_id)
        .execute(&self.pool)
        .await?;
        Ok(id)
    }

    pub async fn insert_meeting(
        &self,
        conversation_id: Uuid,
        lead_id: Option<Uuid>,
        slot_start: DateTime<Utc>,
        slot_end: DateTime<Utc>,
        calendar_event_id: Option<&str>,
        meet_link: Option<&str>,
    ) -> Result<Uuid> {
        let id = Uuid::new_v4();
        sqlx::query(
            r#"INSERT INTO meetings
               (id, conversation_id, lead_id, slot_start, slot_end, calendar_event_id, meet_link)
               VALUES ($1, $2, $3, $4, $5, $6, $7)"#,
        )
        .bind(id)
        .bind(conversation_id)
        .bind(lead_id)
        .bind(slot_start)
        .bind(slot_end)
        .bind(calendar_event_id)
        .bind(meet_link)
        .execute(&self.pool)
        .await?;
        Ok(id)
    }
}

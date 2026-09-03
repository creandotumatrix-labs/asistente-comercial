//! Web demo channel. Lets the landing page talk to the *live* agent directly,
//! with no WhatsApp involved: same brain as the webhook (real Claude, scoring,
//! Google Calendar, HubSpot) — the reply is returned as JSON instead of sent
//! over WhatsApp. Web sessions are namespaced (`web-…`) so they can never
//! collide with real WhatsApp `wa_id`s.

use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::agent;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct ChatReq {
    pub session_id: String,
    pub text: String,
}

#[derive(Serialize)]
pub struct ChatResp {
    pub reply: String,
    pub status: String,
    pub score: Option<String>,
}

pub async fn chat(State(state): State<AppState>, Json(req): Json<ChatReq>) -> Json<ChatResp> {
    // Bound the input; a public endpoint should not forward arbitrarily large
    // prompts to the model.
    let text: String = req.text.chars().take(1000).collect();
    if text.trim().is_empty() {
        return Json(ChatResp {
            reply: "¿Me escribes tu consulta? 🙂".to_string(),
            status: "active".to_string(),
            score: None,
        });
    }

    // Sanitize + namespace the session id into a synthetic contact key.
    let sid: String = req
        .session_id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
        .take(64)
        .collect();
    let sid = if sid.is_empty() {
        Uuid::new_v4().to_string()
    } else {
        sid
    };
    let from = format!("web-{sid}");
    let message_id = format!("{from}:{}", Uuid::new_v4());

    match agent::process_message(
        &state,
        &from,
        Some("Demo web".to_string()),
        text,
        &message_id,
    )
    .await
    {
        Ok(Some(r)) => Json(ChatResp {
            reply: r.reply,
            status: r.status,
            score: r.score,
        }),
        Ok(None) => Json(ChatResp {
            reply: String::new(),
            status: "duplicate".to_string(),
            score: None,
        }),
        Err(e) => {
            tracing::error!("web demo chat failed: {e}");
            Json(ChatResp {
                reply: "Disculpa, tuve un problema técnico. Intenta de nuevo en un momento. 🙏"
                    .to_string(),
                status: "error".to_string(),
                score: None,
            })
        }
    }
}

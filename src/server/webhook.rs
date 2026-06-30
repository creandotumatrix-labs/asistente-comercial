//! Meta WhatsApp webhook: GET verifies the subscription, POST receives events.
//! We ack 200 immediately and process inbound messages on a spawned task —
//! Meta requires a fast response and retries otherwise (we dedupe those).

use axum::body::Bytes;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;

use crate::state::AppState;
use crate::whatsapp::{self, InboundPayload};

#[derive(Debug, Deserialize)]
pub struct VerifyParams {
    #[serde(rename = "hub.mode")]
    mode: Option<String>,
    #[serde(rename = "hub.verify_token")]
    verify_token: Option<String>,
    #[serde(rename = "hub.challenge")]
    challenge: Option<String>,
}

pub async fn verify(State(state): State<AppState>, Query(p): Query<VerifyParams>) -> Response {
    let token_ok = p.verify_token.as_deref() == Some(state.config.whatsapp.verify_token.as_str());
    if p.mode.as_deref() == Some("subscribe") && token_ok {
        if let Some(challenge) = p.challenge {
            // Meta expects the raw challenge echoed back as plain text.
            return (StatusCode::OK, challenge).into_response();
        }
    }
    (StatusCode::FORBIDDEN, "forbidden").into_response()
}

pub async fn receive(State(state): State<AppState>, headers: HeaderMap, body: Bytes) -> StatusCode {
    let raw: &[u8] = body.as_ref();

    // Authenticate the payload against the Meta app secret (when configured).
    if let Some(secret) = state.config.whatsapp.app_secret.as_deref() {
        let sig = headers
            .get("X-Hub-Signature-256")
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default();
        if !whatsapp::verify_signature(secret, raw, sig) {
            tracing::warn!("invalid X-Hub-Signature-256; rejecting webhook");
            return StatusCode::UNAUTHORIZED;
        }
    } else {
        tracing::warn!(
            "WHATSAPP_APP_SECRET not set; skipping webhook signature check (set it for production)"
        );
    }

    let payload: InboundPayload = match serde_json::from_slice(raw) {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!("inbound webhook parse error: {e}");
            return StatusCode::OK; // ack anyway so Meta doesn't hammer retries
        }
    };

    let messages = payload.inbound_messages();
    if messages.is_empty() {
        // Status callbacks (delivered/read) and other non-message events.
        return StatusCode::OK;
    }

    tokio::spawn(async move {
        for msg in messages {
            if let Err(e) = crate::agent::handle_inbound(state.clone(), msg).await {
                tracing::error!("handle_inbound failed: {e}");
            }
        }
    });

    StatusCode::OK
}

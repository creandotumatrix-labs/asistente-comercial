//! Agent orchestrator: the qualify → book → route loop. One inbound message
//! in; one outbound reply out; full transcript + side effects persisted.
//!
//! `process_message` is channel-agnostic — WhatsApp (`handle_inbound`) and the
//! web demo (`server::demo`) both drive the same brain; only delivery differs.

pub mod llm;
pub mod prompt;
pub mod tools;

pub use llm::LlmClient;

use anyhow::Result;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::state::AppState;
use crate::whatsapp::InboundText;
use tools::ToolContext;

/// Cap on LLM⇄tool round-trips per inbound message (prevents runaway loops).
const MAX_STEPS: usize = 8;

/// Outcome of processing one inbound message, independent of the channel that
/// delivered it. The caller decides how to deliver `reply`.
pub struct TurnResult {
    pub reply: String,
    pub status: String,
    pub score: Option<String>,
    pub conversation_id: Uuid,
}

/// Core turn: idempotency + per-contact lock + Anthropic tool-use loop +
/// persistence. Returns the reply to deliver (or `None` for a duplicate). Does
/// NOT send anything itself — WhatsApp send / web JSON is the caller's job.
pub async fn process_message(
    state: &AppState,
    from: &str,
    profile_name: Option<String>,
    text: String,
    message_id: &str,
) -> Result<Option<TurnResult>> {
    // Idempotency: never process the same message id twice (Meta retries; web
    // sends a fresh id per turn).
    if !state.store.mark_message_processed(message_id).await? {
        tracing::info!(message_id = %message_id, "duplicate inbound, skipping");
        return Ok(None);
    }

    // Serialize processing per contact/session so quick successive messages
    // don't race the conversation history.
    let _conv_guard = state.locks.acquire(from).await;

    let conv = state
        .store
        .get_or_create_conversation(from, profile_name.as_deref())
        .await?;

    let mut history = conv.messages();
    history.push(json!({
        "role": "user",
        "content": [{ "type": "text", "text": text }]
    }));

    let mut ctx = ToolContext::new(
        conv.id,
        from.to_string(),
        conv.profile_name.clone().or(profile_name),
    );

    let system = prompt::system_prompt(&state.offer);
    let tools = tools::tool_definitions(&state.offer);

    let mut reply = String::new();
    let mut completed = false;

    for _ in 0..MAX_STEPS {
        let resp = match state.llm.create(&system, &history, &tools).await {
            Ok(r) => r,
            Err(e) => {
                tracing::error!("anthropic call failed: {e}");
                reply =
                    "Disculpa, tuve un problema técnico momentáneo. Un asesor te contactará. 🙏"
                        .to_string();
                completed = true;
                break;
            }
        };
        history.push(resp.assistant_message.clone());

        if resp.tool_uses.is_empty() {
            reply = resp.text.clone();
            completed = true;
            break;
        }

        let mut results = Vec::with_capacity(resp.tool_uses.len());
        for tu in &resp.tool_uses {
            let content = tools::dispatch(state, &mut ctx, &tu.name, &tu.input).await;
            results.push(json!({
                "type": "tool_result",
                "tool_use_id": tu.id,
                "content": content
            }));
        }
        history.push(json!({ "role": "user", "content": results }));

        // Persist progress so a crash mid-loop doesn't lose state.
        let snapshot = Value::Array(history.clone());
        if let Err(e) = state
            .store
            .save_conversation(conv.id, &snapshot, &ctx.status, ctx.score_str())
            .await
        {
            tracing::error!("save_conversation (mid-loop) failed: {e}");
        }
    }

    if reply.trim().is_empty() {
        reply = "¿Me compartes un dato más para ayudarte mejor? 🙏".to_string();
    }

    // If we exhausted the loop still mid-tool-call, close the turn cleanly so the
    // stored history alternates user/assistant correctly for the next message.
    if !completed {
        history.push(json!({
            "role": "assistant",
            "content": [{ "type": "text", "text": reply }]
        }));
    }

    // Persist the turn BEFORE delivery, so an outbound failure never loses state.
    let final_history = Value::Array(history);
    state
        .store
        .save_conversation(conv.id, &final_history, &ctx.status, ctx.score_str())
        .await?;

    Ok(Some(TurnResult {
        reply,
        status: ctx.status.clone(),
        score: ctx.score_str().map(|s| s.to_string()),
        conversation_id: conv.id,
    }))
}

/// WhatsApp channel: process the message, then deliver the reply over the
/// WhatsApp Cloud API.
pub async fn handle_inbound(state: AppState, msg: InboundText) -> Result<()> {
    let from = msg.from.clone();
    if let Some(res) =
        process_message(&state, &from, msg.profile_name.clone(), msg.text, &msg.message_id).await?
    {
        if let Err(e) = state.whatsapp.send_text(&from, &res.reply).await {
            tracing::error!("whatsapp send failed (turn persisted): {e}");
        }
    }
    Ok(())
}

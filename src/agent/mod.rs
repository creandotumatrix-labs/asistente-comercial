//! Agent orchestrator: the qualify → book → route loop. One inbound WhatsApp
//! message in; one outbound reply out; full transcript + side effects persisted.

pub mod llm;
pub mod prompt;
pub mod tools;

pub use llm::LlmClient;

use anyhow::Result;
use serde_json::{json, Value};

use crate::state::AppState;
use crate::whatsapp::InboundText;
use tools::ToolContext;

/// Cap on LLM⇄tool round-trips per inbound message (prevents runaway loops).
const MAX_STEPS: usize = 8;

pub async fn handle_inbound(state: AppState, msg: InboundText) -> Result<()> {
    // Idempotency: Meta retries webhooks; never process the same message twice.
    if !state.store.mark_message_processed(&msg.message_id).await? {
        tracing::info!(message_id = %msg.message_id, "duplicate inbound, skipping");
        return Ok(());
    }

    // Serialize processing per contact so quick successive messages don't race
    // the conversation history (held until this message's turn completes).
    let _conv_guard = state.locks.acquire(&msg.from).await;

    let conv = state
        .store
        .get_or_create_conversation(&msg.from, msg.profile_name.as_deref())
        .await?;

    let mut history = conv.messages();
    history.push(json!({
        "role": "user",
        "content": [{ "type": "text", "text": msg.text }]
    }));

    let mut ctx = ToolContext::new(
        conv.id,
        msg.from.clone(),
        conv.profile_name.clone().or_else(|| msg.profile_name.clone()),
    );

    let system = prompt::system_prompt(&state.offer);
    let tools = tools::tool_definitions(&state.offer);

    let mut reply = String::new();
    let mut completed = false;

    for _ in 0..MAX_STEPS {
        let resp = state.llm.create(&system, &history, &tools).await?;
        history.push(resp.assistant_message.clone());

        if resp.tool_uses.is_empty() {
            reply = resp.text.clone();
            completed = true;
            break;
        }

        let mut results = Vec::with_capacity(resp.tool_uses.len());
        for tu in &resp.tool_uses {
            let content = tools::dispatch(&state, &mut ctx, &tu.name, &tu.input).await;
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

    state.whatsapp.send_text(&msg.from, &reply).await?;

    let final_history = Value::Array(history);
    state
        .store
        .save_conversation(conv.id, &final_history, &ctx.status, ctx.score_str())
        .await?;
    Ok(())
}

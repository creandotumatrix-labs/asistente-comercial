//! Human-readable transcript view linked from the rep handoff message.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use serde_json::Value;
use uuid::Uuid;

use crate::state::AppState;
use crate::store::Conversation;

pub async fn show(State(state): State<AppState>, Path(id): Path<Uuid>) -> Response {
    match state.store.get_conversation(id).await {
        Ok(Some(conv)) => Html(render(&conv)).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "Conversación no encontrada").into_response(),
        Err(e) => {
            tracing::error!("transcript fetch failed: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "error").into_response()
        }
    }
}

fn render(conv: &Conversation) -> String {
    let mut rows = String::new();
    for m in conv.messages() {
        let role = m.get("role").and_then(|r| r.as_str()).unwrap_or("?");
        let Some(content) = m.get("content").and_then(|c| c.as_array()) else {
            continue;
        };
        for block in content {
            match block.get("type").and_then(|t| t.as_str()) {
                Some("text") => {
                    let t = block.get("text").and_then(|x| x.as_str()).unwrap_or("");
                    if !t.trim().is_empty() {
                        rows.push_str(&bubble(role, &esc(t)));
                    }
                }
                Some("tool_use") => {
                    let n = block.get("name").and_then(|x| x.as_str()).unwrap_or("");
                    let input = block.get("input").map(Value::to_string).unwrap_or_default();
                    rows.push_str(&bubble("tool", &format!("🔧 <b>{}</b>({})", esc(n), esc(&input))));
                }
                Some("tool_result") => {
                    let c = block
                        .get("content")
                        .map(|v| match v {
                            Value::String(s) => s.clone(),
                            other => other.to_string(),
                        })
                        .unwrap_or_default();
                    rows.push_str(&bubble("tool", &format!("↩︎ {}", esc(&c))));
                }
                _ => {}
            }
        }
    }

    let score = conv.score.as_deref().unwrap_or("—");
    format!(
        r#"<!doctype html>
<html lang="es"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Transcripción {id}</title>
<style>
  body {{ font: 15px/1.5 -apple-system, system-ui, sans-serif; background:#0b141a; color:#e9edef; margin:0; padding:24px; }}
  .wrap {{ max-width: 720px; margin: 0 auto; }}
  header {{ margin-bottom:16px; }}
  .meta {{ color:#8696a0; font-size:13px; }}
  .b {{ padding:8px 12px; border-radius:10px; margin:6px 0; max-width:80%; white-space:pre-wrap; word-wrap:break-word; }}
  .user {{ background:#202c33; }}
  .assistant {{ background:#005c4b; margin-left:auto; }}
  .tool {{ background:#11202a; color:#8696a0; font-family:ui-monospace, monospace; font-size:12px; }}
  .role {{ font-size:11px; color:#8696a0; margin:10px 0 0; }}
</style></head>
<body><div class="wrap">
<header><h2>Transcripción del lead</h2>
<div class="meta">ID {id} · Estado: {status} · Puntaje: {score}</div></header>
{rows}
</div></body></html>"#,
        id = conv.id,
        status = esc(&conv.status),
        score = esc(score),
        rows = rows
    )
}

fn bubble(role: &str, html: &str) -> String {
    let (cls, who) = match role {
        "user" => ("user", "Prospecto"),
        "assistant" => ("assistant", "Asistente"),
        _ => ("tool", "Sistema"),
    };
    format!(r#"<div class="role">{who}</div><div class="b {cls}">{html}</div>"#)
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

//! Tool schemas (driven by `offer.json`) and the dispatch that executes them
//! against the real backends. Tool failures are returned as `{"error": ...}`
//! content rather than propagated, so a transient backend hiccup degrades into
//! the agent apologizing — not a dropped conversation.

use chrono::{DateTime, Utc};
use serde_json::{json, Map, Value};
use uuid::Uuid;

use crate::calendar::BookRequest;
use crate::config::{Offer, Slot, SlotKind};
use crate::crm::LeadInput;
use crate::notify::HandoffPayload;
use crate::scoring::{self, ScoreOutput};
use crate::state::AppState;

/// Mutable per-turn scratchpad threaded through the tool calls of one inbound
/// message, so e.g. `handoff_human` can reuse the score `score_lead` computed.
pub struct ToolContext {
    pub conversation_id: Uuid,
    pub wa_id: String,
    pub profile_name: Option<String>,
    pub last_score: Option<ScoreOutput>,
    pub last_fields: Value,
    pub last_lead_id: Option<Uuid>,
    pub last_name: Option<String>,
    pub last_contact: Option<String>,
    pub last_meeting_label: Option<String>,
    pub status: String,
}

impl ToolContext {
    pub fn new(conversation_id: Uuid, wa_id: String, profile_name: Option<String>) -> Self {
        Self {
            conversation_id,
            wa_id,
            profile_name,
            last_score: None,
            last_fields: json!({}),
            last_lead_id: None,
            last_name: None,
            last_contact: None,
            last_meeting_label: None,
            status: "active".to_string(),
        }
    }

    pub fn score_str(&self) -> Option<&str> {
        self.last_score.as_ref().map(|s| s.score.as_str())
    }
}

/// Build the tool list for the Anthropic request. `score_lead`'s schema is
/// generated from the offer's slots, so reconfiguring slots reconfigures the tool.
pub fn tool_definitions(offer: &Offer) -> Vec<Value> {
    let mut score_props = Map::new();
    for s in &offer.qualification.slots {
        score_props.insert(s.key.clone(), slot_property(s));
    }
    let required = offer.required_slot_keys();

    vec![
        json!({
            "name": "score_lead",
            "description": "Califica y puntúa al prospecto con los datos recolectados. \
                Llama SOLO cuando tengas los campos obligatorios. Devuelve score (hot|warm|cold) y razones. \
                El puntaje lo calcula la rúbrica; no lo inventes.",
            "input_schema": {
                "type": "object",
                "properties": score_props,
                "required": required
            }
        }),
        json!({
            "name": "get_availability",
            "description": "Devuelve horarios reales disponibles en la agenda del asesor. \
                Úsalo siempre antes de proponer un horario.",
            "input_schema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "book_meeting",
            "description": "Agenda una cita real en el calendario. Usa el valor 'slot' (RFC3339) \
                devuelto por get_availability. Incluye nombre y correo del prospecto si los tienes \
                (para enviarle la invitación).",
            "input_schema": {
                "type": "object",
                "properties": {
                    "slot": { "type": "string", "description": "Inicio del horario en formato RFC3339, tomado de get_availability" },
                    "name": { "type": "string", "description": "Nombre del prospecto" },
                    "email": { "type": "string", "description": "Correo del prospecto para la invitación" }
                },
                "required": ["slot"]
            }
        }),
        json!({
            "name": "create_lead",
            "description": "Registra el lead calificado en el CRM. Confirma nombre y contacto antes de llamar.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "name": { "type": "string" },
                    "contact": { "type": "string", "description": "Correo o teléfono" },
                    "email": { "type": "string" },
                    "phone": { "type": "string" },
                    "score": { "type": "string", "enum": ["hot", "warm", "cold"] },
                    "qualified_fields": { "type": "object", "description": "Valores calificados (opcional; si se omite se usan los de score_lead)" }
                },
                "required": []
            }
        }),
        json!({
            "name": "handoff_human",
            "description": "Entrega el lead al asesor humano con puntaje, razones, contacto y enlace a la transcripción.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "score": { "type": "string", "enum": ["hot", "warm", "cold"] },
                    "summary": { "type": "string", "description": "Resumen breve para el asesor" }
                },
                "required": ["summary"]
            }
        }),
        json!({
            "name": "send_resource",
            "description": "Comparte un recurso/enlace útil (FAQ, guía) para desviar leads fríos sin ocupar la agenda.",
            "input_schema": {
                "type": "object",
                "properties": {
                    "topic": { "type": "string", "description": "Tema del recurso; usa 'default' si no aplica" }
                },
                "required": []
            }
        }),
    ]
}

/// Execute a tool call. Always returns a JSON string for the tool_result block.
pub async fn dispatch(
    state: &AppState,
    ctx: &mut ToolContext,
    name: &str,
    input: &Value,
) -> String {
    match name {
        "score_lead" => {
            let out = scoring::score(&state.offer, input);
            ctx.last_fields = input.clone();
            let result = json!({
                "score": out.score.as_str(),
                "score_value": out.score_value,
                "reasons": out.reasons,
                "disqualified": out.disqualified,
            });
            ctx.last_score = Some(out);
            if ctx.status == "active" {
                ctx.status = "qualified".to_string();
            }
            result.to_string()
        }

        "get_availability" => match state.calendar.get_availability(&state.offer.calendar).await {
            Ok(slots) => {
                let arr: Vec<Value> = slots
                    .iter()
                    .map(|s| json!({ "slot": s.start.to_rfc3339(), "label": s.label }))
                    .collect();
                json!({ "slots": arr, "timezone": state.offer.calendar.timezone }).to_string()
            }
            Err(e) => err("get_availability", &e.to_string()),
        },

        "book_meeting" => {
            let slot_str = input
                .get("slot")
                .and_then(|x| x.as_str())
                .unwrap_or_default();
            let start = match DateTime::parse_from_rfc3339(slot_str) {
                Ok(d) => d.with_timezone(&Utc),
                Err(_) => {
                    return err(
                        "book_meeting",
                        "Falta o es inválido 'slot'. Usa exactamente el valor 'slot' de get_availability.",
                    )
                }
            };
            let name = input
                .get("name")
                .and_then(|x| x.as_str())
                .map(str::to_string)
                .or_else(|| ctx.profile_name.clone());
            let email = input
                .get("email")
                .and_then(|x| x.as_str())
                .map(str::to_string);

            let req = BookRequest {
                start,
                summary: format!(
                    "{} · cita con {}",
                    state.offer.branding.business_name,
                    name.as_deref().unwrap_or("prospecto")
                ),
                description: describe(&ctx.last_fields, ctx.last_score.as_ref()),
                attendee_name: name.clone(),
                attendee_email: email.clone(),
            };

            match state
                .calendar
                .book_meeting(&state.offer.calendar, req)
                .await
            {
                Ok(b) => {
                    if let Err(e) = state
                        .store
                        .insert_meeting(
                            ctx.conversation_id,
                            ctx.last_lead_id,
                            b.start,
                            b.end,
                            Some(&b.event_id),
                            b.meet_link.as_deref(),
                        )
                        .await
                    {
                        tracing::error!("insert_meeting failed: {e}");
                    }
                    ctx.last_meeting_label = Some(b.label.clone());
                    if let Some(n) = name {
                        ctx.last_name = Some(n);
                    }
                    if let Some(e) = email {
                        ctx.last_contact = Some(e);
                    }
                    ctx.status = "booked".to_string();
                    json!({
                        "ok": true,
                        "start": b.start.to_rfc3339(),
                        "label": b.label,
                        "meet_link": b.meet_link,
                        "event_link": b.html_link
                    })
                    .to_string()
                }
                Err(e) => err("book_meeting", &e.to_string()),
            }
        }

        "create_lead" => {
            let name = input
                .get("name")
                .and_then(|x| x.as_str())
                .map(str::to_string)
                .or_else(|| ctx.last_name.clone())
                .or_else(|| ctx.profile_name.clone());

            let mut email = input
                .get("email")
                .and_then(|x| x.as_str())
                .map(str::to_string);
            let mut phone = input
                .get("phone")
                .and_then(|x| x.as_str())
                .map(str::to_string);
            if let Some(contact) = input.get("contact").and_then(|x| x.as_str()) {
                if contact.contains('@') {
                    email.get_or_insert_with(|| contact.to_string());
                } else if phone.is_none() {
                    phone = Some(contact.to_string());
                }
            }
            phone.get_or_insert_with(|| ctx.wa_id.clone());

            let score = input
                .get("score")
                .and_then(|x| x.as_str())
                .map(str::to_string)
                .or_else(|| ctx.score_str().map(str::to_string))
                .unwrap_or_else(|| "warm".to_string());

            let reasons = ctx
                .last_score
                .as_ref()
                .map(|s| s.reasons.clone())
                .unwrap_or_default();
            let fields = input
                .get("qualified_fields")
                .cloned()
                .unwrap_or_else(|| ctx.last_fields.clone());
            let summary = lead_summary(&fields, ctx.last_meeting_label.as_deref());

            let lead = LeadInput {
                name: name.clone(),
                email: email.clone(),
                phone: phone.clone(),
                score: score.clone(),
                reasons: reasons.clone(),
                qualified_fields: fields.clone(),
                summary,
            };

            let crm_id = match state.crm.create_lead(&lead).await {
                Ok(r) => r.crm_id,
                Err(e) => {
                    tracing::error!("crm create_lead failed: {e}");
                    None
                }
            };

            let reasons_json = json!(reasons);
            match state
                .store
                .insert_lead(
                    ctx.conversation_id,
                    name.as_deref(),
                    email.as_deref(),
                    phone.as_deref(),
                    &score,
                    &reasons_json,
                    &fields,
                    crm_id.as_deref(),
                )
                .await
            {
                Ok(id) => ctx.last_lead_id = Some(id),
                Err(e) => tracing::error!("insert_lead failed: {e}"),
            }
            ctx.last_name = name;
            ctx.last_contact = email.or(phone);

            json!({ "ok": true, "crm_id": crm_id }).to_string()
        }

        "handoff_human" => {
            let summary = input
                .get("summary")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            let score = input
                .get("score")
                .and_then(|x| x.as_str())
                .map(str::to_string)
                .or_else(|| ctx.score_str().map(str::to_string))
                .unwrap_or_else(|| "warm".to_string());

            let transcript_url = format!(
                "{}/conversations/{}/transcript",
                state.config.public_base_url.trim_end_matches('/'),
                ctx.conversation_id
            );
            let reasons = ctx
                .last_score
                .as_ref()
                .map(|s| s.reasons.clone())
                .unwrap_or_default();

            let payload = HandoffPayload {
                score,
                name: ctx.last_name.clone().or_else(|| ctx.profile_name.clone()),
                contact: ctx.last_contact.clone(),
                reasons,
                summary,
                transcript_url,
                meeting_label: ctx.last_meeting_label.clone(),
            };

            if let Err(e) = state.notifier.handoff(&state.offer.handoff, &payload).await {
                tracing::error!("handoff failed: {e}");
                return err("handoff_human", "No se pudo notificar al asesor.");
            }
            json!({ "ok": true }).to_string()
        }

        "send_resource" => {
            let topic = input
                .get("topic")
                .and_then(|x| x.as_str())
                .unwrap_or("default");
            let link = state
                .offer
                .resources
                .get(topic)
                .or_else(|| state.offer.resources.get("default"))
                .cloned();
            if ctx.status != "booked" {
                ctx.status = "deflected".to_string();
            }
            match link {
                Some(l) => json!({ "ok": true, "link": l }).to_string(),
                None => json!({ "ok": false, "note": "sin recurso configurado" }).to_string(),
            }
        }

        other => err(other, "herramienta desconocida"),
    }
}

fn slot_property(slot: &Slot) -> Value {
    let mut p = Map::new();
    let t = match slot.kind {
        SlotKind::Number => "number",
        _ => "string",
    };
    p.insert("type".into(), json!(t));
    p.insert("description".into(), json!(slot.question));
    if matches!(slot.kind, SlotKind::Enum) && !slot.values.is_empty() {
        p.insert("enum".into(), json!(slot.values));
    }
    Value::Object(p)
}

fn describe(fields: &Value, score: Option<&ScoreOutput>) -> String {
    let mut s = String::from("Lead entrante por WhatsApp.\n");
    if let Some(obj) = fields.as_object() {
        for (k, v) in obj {
            s.push_str(&format!("- {k}: {}\n", render_value(v)));
        }
    }
    if let Some(sc) = score {
        s.push_str(&format!("Puntaje: {} ({:.2})\n", sc.score, sc.score_value));
    }
    s
}

fn lead_summary(fields: &Value, meeting: Option<&str>) -> String {
    let mut parts = Vec::new();
    if let Some(obj) = fields.as_object() {
        for (k, v) in obj {
            parts.push(format!("{k}: {}", render_value(v)));
        }
    }
    let mut s = parts.join(" · ");
    if let Some(m) = meeting {
        s.push_str(&format!(" · cita: {m}"));
    }
    s
}

fn render_value(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn err(tool: &str, msg: &str) -> String {
    tracing::warn!("tool `{tool}` error: {msg}");
    json!({ "error": msg }).to_string()
}

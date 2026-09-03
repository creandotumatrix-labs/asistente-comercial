//! WhatsApp Cloud API webhook payloads (inbound) — parsed defensively: Meta
//! sends message events AND status events through the same hook, so almost
//! everything is optional.

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct InboundPayload {
    #[serde(default)]
    pub entry: Vec<Entry>,
}

#[derive(Debug, Deserialize)]
pub struct Entry {
    #[serde(default)]
    pub changes: Vec<Change>,
}

#[derive(Debug, Deserialize)]
pub struct Change {
    #[serde(default)]
    pub value: ChangeValue,
}

#[derive(Debug, Default, Deserialize)]
pub struct ChangeValue {
    // Not read; kept so the field is visible when the value is logged/debugged.
    #[serde(default)]
    #[allow(dead_code)]
    pub messaging_product: Option<String>,
    #[serde(default)]
    pub contacts: Vec<Contact>,
    #[serde(default)]
    pub messages: Vec<Message>,
    // `statuses` (delivery receipts) intentionally ignored.
}

#[derive(Debug, Deserialize)]
pub struct Contact {
    // `InboundText.from` (the message envelope) carries the wa_id used
    // downstream; this copy is unread but kept for parity with the payload.
    #[serde(default)]
    #[allow(dead_code)]
    pub wa_id: String,
    #[serde(default)]
    pub profile: Profile,
}

#[derive(Debug, Default, Deserialize)]
pub struct Profile {
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Message {
    pub id: String,
    pub from: String,
    #[serde(rename = "type", default)]
    pub kind: String,
    #[serde(default)]
    pub text: Option<TextBody>,
}

#[derive(Debug, Deserialize)]
pub struct TextBody {
    #[serde(default)]
    pub body: String,
}

/// Normalized inbound message the agent loop consumes.
#[derive(Debug, Clone)]
pub struct InboundText {
    pub message_id: String,
    pub from: String,
    pub profile_name: Option<String>,
    pub text: String,
}

impl InboundPayload {
    /// Flatten the nested webhook envelope into per-message records. Non-text
    /// messages are surfaced with a placeholder so the agent can ask for text
    /// (and so we still dedupe their id).
    pub fn inbound_messages(&self) -> Vec<InboundText> {
        let mut out = Vec::new();
        for entry in &self.entry {
            for change in &entry.changes {
                let value = &change.value;
                let profile_name = value.contacts.first().and_then(|c| c.profile.name.clone());
                for m in &value.messages {
                    let text = match (m.kind.as_str(), &m.text) {
                        ("text", Some(t)) => t.body.clone(),
                        _ => format!("[mensaje no de texto: {}]", m.kind),
                    };
                    out.push(InboundText {
                        message_id: m.id.clone(),
                        from: m.from.clone(),
                        profile_name: profile_name.clone(),
                        text,
                    });
                }
            }
        }
        out
    }
}

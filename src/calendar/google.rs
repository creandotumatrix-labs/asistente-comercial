//! Google Calendar via a service account (domain-wide-delegation not required
//! if the target calendar is shared with the SA, or the SA owns it).
//!
//! Auth: sign a JWT with the SA private key, exchange it for an access token at
//! Google's OAuth2 token endpoint, cache until expiry. Then call freeBusy +
//! events.insert directly over REST.

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::Serialize;
use serde_json::{json, Value};
use tokio::sync::Mutex;
use uuid::Uuid;

use super::{
    candidate_slots, label_for, subtract_busy, BookRequest, BookedMeeting, Calendar, Slot,
};
use crate::config::CalendarCfg;

const SCOPE: &str = "https://www.googleapis.com/auth/calendar";

pub struct GoogleCalendar {
    http: reqwest::Client,
    client_email: String,
    private_key: String,
    token_uri: String,
    token: Mutex<Option<Cached>>,
}

struct Cached {
    token: String,
    exp: DateTime<Utc>,
}

#[derive(Serialize)]
struct Claims<'a> {
    iss: &'a str,
    scope: &'a str,
    aud: &'a str,
    iat: i64,
    exp: i64,
}

impl GoogleCalendar {
    pub fn new(http: reqwest::Client, service_account_json: &str) -> Result<Self> {
        let v: Value = serde_json::from_str(service_account_json)
            .context("parsing GOOGLE service account json")?;
        let client_email = v
            .get("client_email")
            .and_then(|x| x.as_str())
            .ok_or_else(|| anyhow!("service account json missing `client_email`"))?
            .to_string();
        let private_key = v
            .get("private_key")
            .and_then(|x| x.as_str())
            .ok_or_else(|| anyhow!("service account json missing `private_key`"))?
            .to_string();
        let token_uri = v
            .get("token_uri")
            .and_then(|x| x.as_str())
            .unwrap_or("https://oauth2.googleapis.com/token")
            .to_string();
        Ok(Self {
            http,
            client_email,
            private_key,
            token_uri,
            token: Mutex::new(None),
        })
    }

    async fn access_token(&self) -> Result<String> {
        {
            let guard = self.token.lock().await;
            if let Some(c) = guard.as_ref() {
                if c.exp > Utc::now() + Duration::seconds(60) {
                    return Ok(c.token.clone());
                }
            }
        }

        let now = Utc::now();
        let iat = now.timestamp();
        let claims = Claims {
            iss: &self.client_email,
            scope: SCOPE,
            aud: &self.token_uri,
            iat,
            exp: iat + 3600,
        };
        let jwt = encode(
            &Header::new(Algorithm::RS256),
            &claims,
            &EncodingKey::from_rsa_pem(self.private_key.as_bytes())
                .context("loading RSA private key (expects PKCS#8 PEM from the SA json)")?,
        )?;

        let resp = self
            .http
            .post(&self.token_uri)
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
                ("assertion", jwt.as_str()),
            ])
            .send()
            .await?;
        let status = resp.status();
        let body: Value = resp.json().await.unwrap_or(Value::Null);
        if !status.is_success() {
            return Err(anyhow!("google token exchange failed ({status}): {body}"));
        }
        let access = body
            .get("access_token")
            .and_then(|x| x.as_str())
            .ok_or_else(|| anyhow!("no access_token in google response: {body}"))?
            .to_string();
        let expires_in = body
            .get("expires_in")
            .and_then(|x| x.as_i64())
            .unwrap_or(3600);

        let mut guard = self.token.lock().await;
        *guard = Some(Cached {
            token: access.clone(),
            exp: now + Duration::seconds(expires_in),
        });
        Ok(access)
    }

    async fn insert_event(
        &self,
        token: &str,
        url: &str,
        body: &Value,
    ) -> Result<(reqwest::StatusCode, Value)> {
        let resp = self
            .http
            .post(url)
            .bearer_auth(token)
            .json(body)
            .send()
            .await?;
        let status = resp.status();
        let v: Value = resp.json().await.unwrap_or(Value::Null);
        Ok((status, v))
    }

    fn to_booked(
        cfg: &CalendarCfg,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        v: &Value,
    ) -> BookedMeeting {
        let event_id = v
            .get("id")
            .and_then(|x| x.as_str())
            .unwrap_or_default()
            .to_string();
        let html_link = v
            .get("htmlLink")
            .and_then(|x| x.as_str())
            .map(str::to_string);
        let meet_link = v
            .get("hangoutLink")
            .and_then(|x| x.as_str())
            .map(str::to_string)
            .or_else(|| extract_video_entry_point(v));
        BookedMeeting {
            event_id,
            start,
            end,
            html_link,
            meet_link,
            label: label_for(cfg, start),
        }
    }
}

#[async_trait]
impl Calendar for GoogleCalendar {
    async fn get_availability(&self, cfg: &CalendarCfg) -> Result<Vec<Slot>> {
        let token = self.access_token().await?;
        let now = Utc::now();
        let body = json!({
            "timeMin": now.to_rfc3339(),
            "timeMax": (now + Duration::days(cfg.days_ahead + 1)).to_rfc3339(),
            "timeZone": cfg.timezone,
            "items": [{ "id": cfg.calendar_id }]
        });

        let resp = self
            .http
            .post("https://www.googleapis.com/calendar/v3/freeBusy")
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await?;
        let status = resp.status();
        let v: Value = resp.json().await.unwrap_or(Value::Null);
        if !status.is_success() {
            return Err(anyhow!("freeBusy failed ({status}): {v}"));
        }

        let mut busy: Vec<(DateTime<Utc>, DateTime<Utc>)> = Vec::new();
        if let Some(arr) = v
            .get("calendars")
            .and_then(|c| c.get(cfg.calendar_id.as_str()))
            .and_then(|c| c.get("busy"))
            .and_then(|b| b.as_array())
        {
            for item in arr {
                let (Some(s), Some(e)) = (
                    item.get("start").and_then(|x| x.as_str()),
                    item.get("end").and_then(|x| x.as_str()),
                ) else {
                    continue;
                };
                if let (Ok(sd), Ok(ed)) = (
                    DateTime::parse_from_rfc3339(s),
                    DateTime::parse_from_rfc3339(e),
                ) {
                    busy.push((sd.with_timezone(&Utc), ed.with_timezone(&Utc)));
                }
            }
        }

        let slots = candidate_slots(cfg, now)?;
        Ok(subtract_busy(slots, &busy, cfg.max_slots))
    }

    async fn book_meeting(&self, cfg: &CalendarCfg, req: BookRequest) -> Result<BookedMeeting> {
        let token = self.access_token().await?;
        let end = req.start + Duration::minutes(cfg.slot_minutes);

        // Preferred path: real Google Meet link + attendee invite. Works when the
        // target calendar is a Workspace calendar with domain-wide delegation.
        let mut event = json!({
            "summary": req.summary,
            "description": req.description,
            "start": { "dateTime": req.start.to_rfc3339(), "timeZone": cfg.timezone },
            "end":   { "dateTime": end.to_rfc3339(), "timeZone": cfg.timezone },
            "conferenceData": {
                "createRequest": {
                    "requestId": Uuid::new_v4().to_string(),
                    "conferenceSolutionKey": { "type": "hangoutsMeet" }
                }
            }
        });
        if let Some(email) = &req.attendee_email {
            event["attendees"] = json!([{ "email": email, "displayName": req.attendee_name }]);
        }
        let url = format!(
            "https://www.googleapis.com/calendar/v3/calendars/{}/events?conferenceDataVersion=1&sendUpdates=all",
            encode_path(&cfg.calendar_id)
        );

        let (status, v) = self.insert_event(&token, &url, &event).await?;
        if status.is_success() {
            return Ok(Self::to_booked(cfg, req.start, end, &v));
        }

        // A plain service account (no domain-wide delegation) can neither invite
        // attendees nor create Meet conferences on a shared/personal calendar:
        // Google returns 403 forbiddenForServiceAccounts. Fall back to a plain
        // event so the booking still lands and blocks the rep's time; the
        // prospect's contact goes in the body and they get the WhatsApp
        // confirmation instead of a Google invite email.
        if status.as_u16() == 403 {
            tracing::warn!(
                "calendar insert 403 (service account without DWD); retrying plain event: {v}"
            );
            let plain = json!({
                "summary": req.summary,
                "description": with_contact(&req),
                "start": { "dateTime": req.start.to_rfc3339(), "timeZone": cfg.timezone },
                "end":   { "dateTime": end.to_rfc3339(), "timeZone": cfg.timezone }
            });
            let plain_url = format!(
                "https://www.googleapis.com/calendar/v3/calendars/{}/events?sendUpdates=none",
                encode_path(&cfg.calendar_id)
            );
            let (status2, v2) = self.insert_event(&token, &plain_url, &plain).await?;
            if status2.is_success() {
                return Ok(Self::to_booked(cfg, req.start, end, &v2));
            }
            return Err(anyhow!("event insert failed (plain retry {status2}): {v2}"));
        }

        Err(anyhow!("event insert failed ({status}): {v}"))
    }
}

/// Append the prospect's name/email to the event description (used on the
/// plain-event fallback path, where they can't be added as a real attendee).
fn with_contact(req: &BookRequest) -> String {
    let mut s = req.description.clone();
    if req.attendee_name.is_some() || req.attendee_email.is_some() {
        s.push_str("\n\nProspecto: ");
        if let Some(n) = &req.attendee_name {
            s.push_str(n);
        }
        if let Some(e) = &req.attendee_email {
            s.push_str(" <");
            s.push_str(e);
            s.push('>');
        }
    }
    s
}

fn extract_video_entry_point(v: &Value) -> Option<String> {
    v.get("conferenceData")
        .and_then(|c| c.get("entryPoints"))
        .and_then(|e| e.as_array())
        .and_then(|arr| {
            arr.iter()
                .find(|ep| ep.get("entryPointType").and_then(|t| t.as_str()) == Some("video"))
                .and_then(|ep| ep.get("uri").and_then(|u| u.as_str()))
                .map(str::to_string)
        })
}

/// Minimal percent-encoding for a calendar id used in a URL path segment
/// (handles email-style ids containing `@`).
fn encode_path(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

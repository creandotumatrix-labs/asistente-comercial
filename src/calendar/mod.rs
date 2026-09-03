//! Calendar abstraction + slot math. The Google implementation lives in
//! `google.rs`; the trait keeps the agent decoupled from the provider.

pub mod google;
pub use google::GoogleCalendar;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::{DateTime, Datelike, Duration, TimeZone, Timelike, Utc};
use chrono_tz::Tz;
use serde::Serialize;

use crate::config::CalendarCfg;

#[derive(Debug, Clone, Serialize)]
pub struct Slot {
    /// RFC3339 when serialized — the booking key passed back to `book_meeting`.
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    /// Human, localized label, e.g. "jueves 2 jul, 4:30 pm".
    pub label: String,
}

#[derive(Debug, Clone)]
pub struct BookRequest {
    pub start: DateTime<Utc>,
    pub summary: String,
    pub description: String,
    pub attendee_name: Option<String>,
    pub attendee_email: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BookedMeeting {
    pub event_id: String,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub html_link: Option<String>,
    pub meet_link: Option<String>,
    pub label: String,
}

#[async_trait]
pub trait Calendar: Send + Sync {
    /// Free slots within working hours over the configured horizon.
    async fn get_availability(&self, cfg: &CalendarCfg) -> Result<Vec<Slot>>;
    /// Create a real calendar event (with a Meet link) at the chosen slot.
    async fn book_meeting(&self, cfg: &CalendarCfg, req: BookRequest) -> Result<BookedMeeting>;
}

/// Generate candidate slots within working hours over the horizon, honoring the
/// timezone, bookable weekdays, and minimum lead time. Provider impls subtract
/// busy intervals from this set.
pub fn candidate_slots(cfg: &CalendarCfg, now: DateTime<Utc>) -> Result<Vec<Slot>> {
    let tz: Tz = cfg
        .timezone
        .parse()
        .map_err(|_| anyhow!("invalid IANA timezone `{}`", cfg.timezone))?;
    let (sh, sm) = parse_hhmm(&cfg.working_hours.start)?;
    let (eh, em) = parse_hhmm(&cfg.working_hours.end)?;

    let now_local = now.with_timezone(&tz);
    let earliest = now + Duration::minutes(cfg.min_lead_minutes);

    let mut slots = Vec::new();
    for offset in 0..=cfg.days_ahead {
        let date = now_local.date_naive() + Duration::days(offset);
        let iso_weekday = date.weekday().number_from_monday();
        if !cfg.days.contains(&iso_weekday) {
            continue;
        }
        let Some(mut cur) = tz
            .with_ymd_and_hms(date.year(), date.month(), date.day(), sh, sm, 0)
            .single()
        else {
            continue;
        };
        let Some(day_end) = tz
            .with_ymd_and_hms(date.year(), date.month(), date.day(), eh, em, 0)
            .single()
        else {
            continue;
        };

        while cur + Duration::minutes(cfg.slot_minutes) <= day_end {
            let start_utc = cur.with_timezone(&Utc);
            let end_local = cur + Duration::minutes(cfg.slot_minutes);
            if start_utc >= earliest {
                slots.push(Slot {
                    start: start_utc,
                    end: end_local.with_timezone(&Utc),
                    label: label_es(&cur),
                });
            }
            cur = end_local;
        }
    }
    Ok(slots)
}

/// Keep only slots that don't overlap any busy interval; cap at `max_slots`.
pub fn subtract_busy(
    slots: Vec<Slot>,
    busy: &[(DateTime<Utc>, DateTime<Utc>)],
    max: usize,
) -> Vec<Slot> {
    slots
        .into_iter()
        .filter(|s| !busy.iter().any(|(bs, be)| s.start < *be && *bs < s.end))
        .take(max)
        .collect()
}

fn parse_hhmm(s: &str) -> Result<(u32, u32)> {
    let (h, m) = s
        .split_once(':')
        .ok_or_else(|| anyhow!("invalid HH:MM `{s}`"))?;
    Ok((h.trim().parse()?, m.trim().parse()?))
}

fn label_es(dt: &DateTime<Tz>) -> String {
    const WD: [&str; 7] = [
        "lunes",
        "martes",
        "miércoles",
        "jueves",
        "viernes",
        "sábado",
        "domingo",
    ];
    const MO: [&str; 12] = [
        "ene", "feb", "mar", "abr", "may", "jun", "jul", "ago", "sep", "oct", "nov", "dic",
    ];
    let wd = WD[dt.weekday().num_days_from_monday() as usize];
    let mo = MO[dt.month0() as usize];
    let (ampm, h12) = to_ampm(dt.hour());
    format!("{wd} {} {mo}, {h12}:{:02} {ampm}", dt.day(), dt.minute())
}

fn to_ampm(h24: u32) -> (&'static str, u32) {
    let ampm = if h24 < 12 { "am" } else { "pm" };
    let mut h12 = h24 % 12;
    if h12 == 0 {
        h12 = 12;
    }
    (ampm, h12)
}

/// Localized label for an arbitrary instant (used by booking confirmations).
pub fn label_for(cfg: &CalendarCfg, when: DateTime<Utc>) -> String {
    match cfg.timezone.parse::<Tz>() {
        Ok(tz) => label_es(&when.with_timezone(&tz)),
        Err(_) => when.to_rfc3339(),
    }
}

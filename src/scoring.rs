//! Deterministic, auditable lead scoring.
//!
//! The LLM's job is to *extract* structured slot values from the conversation.
//! It does NOT decide the score. This engine applies the configurable rubric in
//! `offer.json` (weights, scales, disqualifiers) to those slots and returns a
//! tier + an ordered, human-readable list of reasons. That makes every score
//! reproducible and defensible — the rep can see exactly why a lead is "hot".

use serde::Serialize;
use serde_json::{Map, Value};

use crate::config::{Offer, Rubric};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Tier {
    Hot,
    Warm,
    Cold,
}

impl Tier {
    pub fn as_str(&self) -> &'static str {
        match self {
            Tier::Hot => "hot",
            Tier::Warm => "warm",
            Tier::Cold => "cold",
        }
    }
    fn label_es(&self) -> &'static str {
        match self {
            Tier::Hot => "caliente",
            Tier::Warm => "tibio",
            Tier::Cold => "frío",
        }
    }
}

impl std::fmt::Display for Tier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct FactorScore {
    pub factor: String,
    pub weight: f64,
    pub raw: f64,
    pub contribution: f64,
    pub note: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScoreOutput {
    pub score: Tier,
    pub score_value: f64,
    pub reasons: Vec<String>,
    pub factors: Vec<FactorScore>,
    pub disqualified: bool,
}

/// Score a set of extracted slot values against the offer's rubric.
pub fn score(offer: &Offer, fields: &Value) -> ScoreOutput {
    let rubric = &offer.qualification.rubric;
    let empty = Map::new();
    let obj = fields.as_object().unwrap_or(&empty);

    let mut reasons: Vec<String> = Vec::new();

    // ── 1) Disqualifiers ────────────────────────────────────────────────────
    let mut soft_penalty = 0.0_f64;
    for dq in &offer.qualification.disqualifiers {
        if !disqualifier_matches(dq, obj) {
            continue;
        }
        if dq.soft {
            soft_penalty += 0.2;
            reasons.push(format!("⚠️ {} (señal suave −0.20)", dq.reason));
        } else {
            // Hard disqualifier: force cold, short-circuit.
            reasons.insert(0, format!("⛔ Descalificado: {}", dq.reason));
            return ScoreOutput {
                score: Tier::Cold,
                score_value: 0.0,
                reasons,
                factors: Vec::new(),
                disqualified: true,
            };
        }
    }

    // ── 2) Weighted factors ─────────────────────────────────────────────────
    let mut factors: Vec<FactorScore> = Vec::new();
    let mut weighted = 0.0_f64;
    let mut total_weight = 0.0_f64;
    for (factor, &weight) in &rubric.weights {
        if weight <= 0.0 {
            continue;
        }
        let (raw, note) = factor_score(factor, obj, rubric);
        weighted += raw * weight;
        total_weight += weight;
        factors.push(FactorScore {
            factor: factor.clone(),
            weight,
            raw,
            contribution: raw * weight,
            note,
        });
    }

    let mut value = if total_weight > 0.0 {
        weighted / total_weight
    } else {
        0.0
    };
    value -= soft_penalty;
    value = value.clamp(0.0, 1.0);

    let t = &rubric.thresholds;
    let tier = if value >= t.hot {
        Tier::Hot
    } else if value >= t.warm {
        Tier::Warm
    } else {
        Tier::Cold
    };

    // Headline first, then factor breakdown ordered by contribution.
    let mut breakdown = factors.clone();
    breakdown.sort_by(|a, b| {
        b.contribution
            .partial_cmp(&a.contribution)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for fct in &breakdown {
        reasons.push(format!(
            "{} (peso {:.2} × {:.2} = {:.2})",
            fct.note, fct.weight, fct.raw, fct.contribution
        ));
    }
    reasons.insert(
        0,
        format!(
            "Puntaje {:.2} → {} (umbrales: hot ≥ {:.2}, warm ≥ {:.2})",
            value,
            tier.label_es(),
            t.hot,
            t.warm
        ),
    );

    ScoreOutput {
        score: tier,
        score_value: round2(value),
        reasons,
        factors,
        disqualified: false,
    }
}

fn disqualifier_matches(dq: &crate::config::Disqualifier, obj: &Map<String, Value>) -> bool {
    let val = field_str(obj, &dq.slot).to_lowercase();
    if let Some(eq) = &dq.equals {
        if val == eq.to_lowercase() {
            return true;
        }
    }
    dq.contains_any
        .iter()
        .any(|needle| !needle.is_empty() && val.contains(&needle.to_lowercase()))
}

fn factor_score(factor: &str, obj: &Map<String, Value>, r: &Rubric) -> (f64, String) {
    match factor {
        "timeline" => {
            let raw_val = field_str(obj, "timeline");
            let key = raw_val.trim().to_lowercase();
            let s = lookup_timeline(&key, r).unwrap_or(r.default_factor_score);
            let shown = if raw_val.is_empty() { "sin dato" } else { &raw_val };
            (s, format!("Plazo: «{shown}»"))
        }
        "size" => match field_num(obj, "size") {
            Some(n) => {
                let s = size_score(n, r);
                (s, format!("Tamaño/volumen: {}", trim_num(n)))
            }
            None => (
                r.default_factor_score,
                "Tamaño/volumen: sin dato".to_string(),
            ),
        },
        "need_fit" => {
            let mut text = field_str(obj, "need");
            text.push(' ');
            text.push_str(&field_str(obj, "context"));
            let text = text.to_lowercase();
            let mut best = 0.0_f64;
            let mut hit: Option<&String> = None;
            for (kw, &w) in &r.need_keywords {
                if !kw.is_empty() && text.contains(&kw.to_lowercase()) && w > best {
                    best = w;
                    hit = Some(kw);
                }
            }
            if let Some(kw) = hit {
                (best, format!("Encaje de necesidad: «{kw}»"))
            } else {
                (
                    r.default_factor_score,
                    "Encaje de necesidad: sin palabra clave".to_string(),
                )
            }
        }
        // Generic numeric factor in [0,1] read directly from the same-named slot.
        other => match field_num(obj, other) {
            Some(n) => (n.clamp(0.0, 1.0), format!("{other}: {}", trim_num(n))),
            None => (r.default_factor_score, format!("{other}: sin dato")),
        },
    }
}

fn lookup_timeline(key: &str, r: &Rubric) -> Option<f64> {
    if let Some(v) = r.timeline_scores.get(key) {
        return Some(*v);
    }
    // Tolerant fallback: substring match against configured keys.
    r.timeline_scores
        .iter()
        .find(|(k, _)| !k.is_empty() && (key.contains(k.as_str()) || k.contains(key)))
        .map(|(_, v)| *v)
}

fn size_score(n: f64, r: &Rubric) -> f64 {
    // Highest satisfied `min` wins.
    let mut best: Option<&crate::config::SizeTier> = None;
    for tier in &r.size_tiers {
        if n >= tier.min && best.map(|b| tier.min > b.min).unwrap_or(true) {
            best = Some(tier);
        }
    }
    best.map(|t| t.score).unwrap_or(r.default_factor_score)
}

fn field_str(obj: &Map<String, Value>, key: &str) -> String {
    match obj.get(key) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Bool(b)) => b.to_string(),
        _ => String::new(),
    }
}

fn field_num(obj: &Map<String, Value>, key: &str) -> Option<f64> {
    match obj.get(key) {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => {
            let digits: String = s
                .chars()
                .skip_while(|c| !c.is_ascii_digit() && *c != '-')
                .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
                .collect();
            digits.parse::<f64>().ok()
        }
        _ => None,
    }
}

fn trim_num(n: f64) -> String {
    if (n.fract()).abs() < f64::EPSILON {
        format!("{}", n as i64)
    } else {
        format!("{n}")
    }
}

fn round2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Offer;
    use serde_json::json;

    fn test_offer() -> Offer {
        serde_json::from_value(json!({
            "branding": { "business_name": "Demo", "agent_name": "Asistente" },
            "offer": { "summary": "Consultoría de automatización" },
            "qualification": {
                "slots": [
                    { "key": "need", "question": "¿Qué necesitas?", "required": true },
                    { "key": "context", "question": "¿Para qué empresa?", "required": true },
                    { "key": "size", "question": "¿Cuántas sucursales?", "required": true, "type": "number" },
                    { "key": "timeline", "question": "¿Para cuándo?", "required": true, "type": "enum",
                      "values": ["this_month","this_quarter","later","unknown"] }
                ],
                "rubric": {
                    "weights": { "size": 0.3, "timeline": 0.4, "need_fit": 0.3 },
                    "timeline_scores": { "this_month": 1.0, "this_quarter": 0.6, "later": 0.2, "unknown": 0.3 },
                    "size_tiers": [
                        { "min": 5, "score": 1.0 },
                        { "min": 2, "score": 0.6 },
                        { "min": 0, "score": 0.3 }
                    ],
                    "need_keywords": {
                        "automatizar": 1.0,
                        "atención a clientes": 1.0,
                        "agendar": 0.8,
                        "ventas": 0.7
                    },
                    "thresholds": { "hot": 0.7, "warm": 0.4 },
                    "default_factor_score": 0.3
                },
                "disqualifiers": [
                    { "slot": "context", "contains_any": ["tarea", "escuela", "estudiante"],
                      "reason": "Consulta escolar, no es una empresa" },
                    { "slot": "timeline", "equals": "later", "soft": true,
                      "reason": "Plazo lejano" }
                ]
            },
            "calendar": {
                "calendar_id": "primary",
                "timezone": "America/Mexico_City",
                "working_hours": { "start": "10:00", "end": "18:00" }
            },
            "resources": { "default": "https://example.com/guia" },
            "deflection": { "message": "Te comparto un recurso útil." },
            "handoff": { "channel": "whatsapp", "rep_wa_id": "5215500000000" }
        }))
        .expect("test offer parses")
    }

    #[test]
    fn hot_lead_books() {
        let offer = test_offer();
        let out = score(
            &offer,
            &json!({
                "need": "automatizar atención a clientes",
                "context": "clínica dental",
                "size": 3,
                "timeline": "this_month"
            }),
        );
        // size=3 → 0.6×0.3=0.18 ; timeline this_month 1.0×0.4=0.40 ; need automatizar 1.0×0.3=0.30
        // total = 0.88 → hot
        assert_eq!(out.score, Tier::Hot, "reasons={:?}", out.reasons);
        assert!(out.score_value >= 0.85, "value={}", out.score_value);
        assert!(!out.disqualified);
    }

    #[test]
    fn warm_lead_in_band() {
        let offer = test_offer();
        let out = score(
            &offer,
            &json!({
                "need": "quiero agendar citas",
                "context": "taller mecánico",
                "size": 2,
                "timeline": "this_quarter"
            }),
        );
        // size 0.6×0.3=0.18 ; timeline 0.6×0.4=0.24 ; need agendar 0.8×0.3=0.24 → 0.66 → warm
        assert_eq!(out.score, Tier::Warm, "reasons={:?}", out.reasons);
        assert!(out.score_value >= 0.40 && out.score_value < 0.70);
    }

    #[test]
    fn cold_lead_low_signal() {
        let offer = test_offer();
        let out = score(
            &offer,
            &json!({
                "need": "solo informacion general",
                "context": "negocio pequeño",
                "size": 1,
                "timeline": "unknown"
            }),
        );
        // size 0.3×0.3=0.09 ; timeline 0.3×0.4=0.12 ; need default 0.3×0.3=0.09 → 0.30 → cold
        assert_eq!(out.score, Tier::Cold, "reasons={:?}", out.reasons);
        assert!(!out.disqualified, "low signal is cold but not disqualified");
    }

    #[test]
    fn hard_disqualifier_forces_cold() {
        let offer = test_offer();
        let out = score(
            &offer,
            &json!({
                "need": "automatizar atención a clientes",
                "context": "es una tarea de la escuela",
                "size": 9,
                "timeline": "this_month"
            }),
        );
        assert_eq!(out.score, Tier::Cold);
        assert!(out.disqualified, "school inquiry must hard-disqualify");
        assert_eq!(out.score_value, 0.0);
    }

    #[test]
    fn soft_disqualifier_penalizes_without_failing() {
        let offer = test_offer();
        // Strong otherwise, but timeline=later applies a soft −0.20.
        let out = score(
            &offer,
            &json!({
                "need": "automatizar ventas",
                "context": "cadena de tiendas",
                "size": 8,
                "timeline": "later"
            }),
        );
        // size 1.0×0.3=0.30 ; timeline later 0.2×0.4=0.08 ; need automatizar 1.0×0.3=0.30
        // = 0.68, minus 0.20 soft = 0.48 → warm, not disqualified
        assert!(!out.disqualified);
        assert_eq!(out.score, Tier::Warm, "reasons={:?}", out.reasons);
    }

    #[test]
    fn missing_fields_default_gracefully() {
        let offer = test_offer();
        let out = score(&offer, &json!({}));
        // all factors → default 0.3 → 0.30 → cold, no panic
        assert_eq!(out.score, Tier::Cold);
        assert!(!out.disqualified);
    }
}

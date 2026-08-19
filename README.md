# Asistente Comercial — WhatsApp Lead Qualifier + Booker

Vertical-agnostic WhatsApp agent that captures an inbound lead, qualifies it
against a **configurable** rubric (BANT-style), **scores** it deterministically,
**books** the good ones on a real Google Calendar, and **routes** them to the rep
with a scored summary. Cold/unfit leads get a polite deflection + resource — not
a calendar slot. One binary becomes a new business by swapping `offer.json`.

Built in Rust (axum + tokio). Real integrations: **WhatsApp Cloud API**,
**Anthropic** (agent brain), **Google Calendar**, **HubSpot** (or generic webhook CRM).

---

## Demo

![Demo en vivo — Asistente Comercial](asistente-comercial-demo.gif)

- 🔴 **Demo en vivo:** [asistente-comercial-production.up.railway.app](https://asistente-comercial-production.up.railway.app/)
- 📄 **Detalles:** [asistente-comercial-demo.vercel.app](https://asistente-comercial-demo.vercel.app/)
- ▶️ **Video:** [youtu.be/Idg40dF3FZE](https://youtu.be/Idg40dF3FZE)

---

## Why the scoring is deterministic (the defensible part)

The LLM **extracts** structured slot values from the chat. It does **not** invent
the score. `src/scoring.rs` applies the rubric in `offer.json` (weights, size
tiers, timeline scale, keyword fit, hard/soft disqualifiers) and returns a tier
+ an ordered, human-readable list of reasons. Every "hot" is reproducible and
auditable — the rep sees exactly *why*. Run `cargo test` to see hot/warm/cold +
disqualifier cases.

---

## Architecture

```
 Prospect (WhatsApp)
        │  inbound message (Meta webhook POST)
        ▼
 ┌──────────────────────────────────────────────┐
 │ axum server  /webhook  /conversations/:id/... │
 │   • verify (GET)  • receive (POST, acks 200)  │
 └───────────────┬──────────────────────────────┘
                 │ spawn (dedup by message id)
                 ▼
 ┌──────────────────────────────────────────────┐
 │ agent loop (Anthropic tool-use, es-MX)        │
 │   system prompt + tools built from offer.json │
 └───┬───────┬───────┬───────┬───────┬──────────┘
     │       │       │       │       │
  score_  get_     book_   create_ handoff_  send_
  lead    avail.   meeting lead    human    resource
     │       │       │       │       │        │
   rubric  Google  Google  HubSpot  rep      offer.json
  (Rust)   Calendar Calendar /webhook (WA/web) resources
     └───────┴───────┴───────┴───────┴────────┘
                 │ history + leads + meetings
                 ▼
              Postgres
```

Tools (`offer.json`-driven): `score_lead`, `get_availability`, `book_meeting`,
`create_lead`, `handoff_human`, `send_resource`.

---

## Repo layout

```
src/
  main.rs            bootstrap: config → db → clients → router → serve
  config.rs          AppConfig (env) + Offer (offer.json white-label surface)
  scoring.rs         deterministic rubric engine (+ unit tests)
  agent/             llm.rs (Anthropic) · prompt.rs (es-MX) · tools.rs · mod.rs (loop)
  whatsapp/          client.rs (send) · types.rs (webhook payloads)
  calendar/          mod.rs (trait + slot math) · google.rs (SA JWT + freeBusy + insert)
  crm/               mod.rs (trait) · hubspot.rs · webhook.rs
  notify.rs          rep handoff (WhatsApp/webhook)
  store/             Postgres repo + models
  server/            webhook.rs · transcript.rs · mod.rs (router)
config/offer.example.json   ← copy to offer.json and edit
migrations/0001_init.sql     (embedded at build time)
scripts/smoke.sh             webhook smoke test (verify + optional inbound)
Makefile · Dockerfile · docker-compose.yml · fly.toml · .env.example
```

---

## Prerequisites

- Rust 1.82+ (`rustup` — pinned via `rust-toolchain.toml`)
- Docker + Docker Compose (for Postgres, or `docker compose up`)
- Accounts: Anthropic, Meta (WhatsApp), Google Cloud (service account), HubSpot
- For local webhooks: `ngrok` (or `cloudflared`)

---

## Quickstart (local, full stack)

```bash
cp .env.example .env                 # fill in the keys (see "Credentials" below)
cp config/offer.example.json config/offer.json
mkdir -p secrets && cp /path/to/google-sa.json secrets/google-service-account.json

docker compose up --build            # starts Postgres + the app on :8080
curl localhost:8080/health           # -> ok
```

Run without Docker (Postgres still needed):

```bash
createdb asistente
export DATABASE_URL=postgres://localhost/asistente
cargo run --release
cargo test                           # scoring engine tests
```

---

## Show it tomorrow — the no-provisioning-wait path

The only thing that can block a real demo is Meta business verification. **Skip it**
with the WhatsApp **test number** Meta issues instantly in the dev app (real Cloud
API, can message up to 5 verified recipients — e.g. your own phone).

1. **Expose the app**: `ngrok http 8080` → copy the `https://….ngrok-free.app` URL.
   Set `PUBLIC_BASE_URL` in `.env` to it (used for transcript links). Restart the app.
2. **Meta app**: developers.facebook.com → Create App → add **WhatsApp** product.
   In *API Setup* you get a **temporary access token**, a **test phone number**, and
   its **phone number ID**. Add your personal number under "To".
   - `.env`: `WHATSAPP_ACCESS_TOKEN`, `WHATSAPP_PHONE_NUMBER_ID`.
3. **Configure the webhook**: WhatsApp → *Configuration* → Edit.
   - Callback URL: `https://….ngrok-free.app/webhook`
   - Verify token: any string — must equal `WHATSAPP_VERIFY_TOKEN` in `.env`.
   - Click **Verify and Save** (hits our GET handler), then **Subscribe** to the
     `messages` field.
4. **Message it** from your phone to the test number: *"Hola, vi su anuncio de
   consultoría"*. The agent qualifies → scores → books → routes.

> The temporary token lasts ~24h (fine for a demo). For production, create a System
> User + permanent token and verify the business.

---

## Credentials

### Anthropic
`ANTHROPIC_API_KEY` from console.anthropic.com. `ANTHROPIC_MODEL` defaults to a
current Sonnet; override as needed.

### Google Calendar (service account)
1. Google Cloud Console → enable **Google Calendar API**.
2. Create a **service account**, add a **JSON key**. Save as
   `secrets/google-service-account.json` (or inline into `GOOGLE_SERVICE_ACCOUNT_JSON`).
3. **Share the target calendar** with the service-account email
   (`…@…iam.gserviceaccount.com`), permission **"Make changes to events"**.
4. Set `offer.json → calendar.calendar_id` to that calendar's id (`primary` only works
   for calendars the SA *owns*; for a person's calendar use their address as the id
   and share it). `timezone` is IANA (e.g. `America/Mexico_City`).

Booking creates a real event **with a Google Meet link** and emails the invite to
the prospect when an email was captured.

### HubSpot (CRM)
`CRM_BACKEND=hubspot`. Create a **Private App** (Settings → Integrations) with scopes
`crm.objects.contacts.write` and `crm.objects.deals.write`; put the token in
`HUBSPOT_TOKEN`. New leads upsert a **contact** + create a **deal** (carrying score +
reasons) and associate them. Defaults `HUBSPOT_PIPELINE=default`,
`HUBSPOT_DEALSTAGE=appointmentscheduled` — change to your pipeline's stage ids.

Prefer something else? `CRM_BACKEND=webhook` + `CRM_WEBHOOK_URL=` POSTs the lead JSON
to Zapier/Make/n8n/your API.

### Rep handoff
In `offer.json → handoff`: `channel: "whatsapp"` + `rep_wa_id` (E.164 without `+`)
DMs the rep the scored summary; or `channel: "webhook"` + `webhook_url`. The message
includes a link to `/conversations/:id/transcript`.

---

## White-label: make it a new business in minutes

Everything vertical-specific lives in `config/offer.json`:

- **branding** — business + agent name, locale, tone
- **offer** — summary, what you sell, FAQ, pricing policy (agent never over-quotes)
- **qualification.slots** — the questions to ask (drives the `score_lead` tool schema)
- **qualification.rubric** — `weights`, `timeline_scores`, `size_tiers`,
  `need_keywords`, `thresholds {hot, warm}`, `default_factor_score`
- **qualification.disqualifiers** — hard (force cold) or `soft` (penalty) by slot
  (`contains_any` / `equals`)
- **calendar** — calendar id, timezone, working hours, slot length, horizon, weekdays
- **resources** — links keyed by topic for cold-lead deflection
- **deflection.message** / **handoff** — closing copy + routing target

Swap the file, restart → new vertical (clinic, agency, B2B SaaS, real estate…).
No recompile.

---

## Demo script (the wow)

1. **Hot:** "automatizar atención a clientes", *clínica dental*, *3 sucursales*,
   *este mes* → `score_lead` returns **hot** with reasons → agent offers real slots →
   books → confirms name/email → routes to the rep with the transcript link.
2. **Cold:** "solo estoy viendo precios, tal vez el otro año" → **cold** → agent
   politely shares a resource, **no calendar slot**. It protects the rep's time by
   applying judgment, not just collecting fields.

Open the printed `/conversations/:id/transcript` URL to show the full decision trail.

---

## Build & verify

```bash
make test             # scoring: hot / warm / cold / hard + soft disqualifier / missing fields
make fmt-check clippy # format + lint (clippy treats warnings as errors)
make release          # optimized build
make smoke            # hits /health + /webhook verify handshake (no side effects)
make smoke ARGS=--inbound   # or: ./scripts/smoke.sh --inbound  (full pipeline, live APIs)
```

Inbound webhooks are authenticated with **HMAC-SHA256** (`X-Hub-Signature-256`)
when `WHATSAPP_APP_SECRET` is set; the smoke script signs its sample payload to
match. Per-contact processing is serialized by an in-process keyed lock
(`ConvLocks`).

---

## Known limitations / phase 2

- Replies use the 24h customer-service window (inbound-initiated). Proactive
  reminders outside it need approved **message templates** (`book_meeting` already
  stores the meeting; wiring a template reminder is a small add).
- Single rep / single calendar. Multi-rep round-robin, sequenced follow-ups, and
  lead enrichment are deliberately out of MVP scope.
- `ConvLocks` is in-process; a multi-replica deployment needs a shared lock
  (e.g. Postgres advisory locks) — a drop-in swap behind the same interface.

-- asistente-comercial schema (idempotent; safe to run on every boot)

CREATE TABLE IF NOT EXISTS conversations (
    id            uuid PRIMARY KEY,
    wa_id         text UNIQUE NOT NULL,
    profile_name  text,
    status        text NOT NULL DEFAULT 'active',   -- active | qualified | booked | deflected
    score         text,                             -- hot | warm | cold
    history       jsonb NOT NULL DEFAULT '[]'::jsonb, -- raw Anthropic message array (audit + replay)
    created_at    timestamptz NOT NULL DEFAULT now(),
    updated_at    timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS leads (
    id               uuid PRIMARY KEY,
    conversation_id  uuid NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    name             text,
    email            text,
    phone            text,
    score            text,
    reasons          jsonb NOT NULL DEFAULT '[]'::jsonb,
    fields           jsonb NOT NULL DEFAULT '{}'::jsonb,
    crm_id           text,
    created_at       timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS meetings (
    id                 uuid PRIMARY KEY,
    conversation_id    uuid NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    lead_id            uuid REFERENCES leads(id) ON DELETE SET NULL,
    slot_start         timestamptz NOT NULL,
    slot_end           timestamptz NOT NULL,
    calendar_event_id  text,
    meet_link          text,
    created_at         timestamptz NOT NULL DEFAULT now()
);

-- Idempotency for Meta webhook redeliveries.
CREATE TABLE IF NOT EXISTS processed_messages (
    message_id  text PRIMARY KEY,
    created_at  timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_leads_conversation ON leads(conversation_id);
CREATE INDEX IF NOT EXISTS idx_meetings_conversation ON meetings(conversation_id);

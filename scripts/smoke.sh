#!/usr/bin/env bash
# Smoke test for the Asistente Comercial webhook.
#
#   ./scripts/smoke.sh            verify handshake only (no side effects)
#   ./scripts/smoke.sh --inbound  also POST a sample inbound message
#                                 (WARNING: triggers live Anthropic + Calendar/CRM
#                                  + a WhatsApp send attempt)
#
# Env: BASE_URL (default http://localhost:8080). Reads .env for
# WHATSAPP_VERIFY_TOKEN and WHATSAPP_APP_SECRET if present.
set -euo pipefail

BASE="${BASE_URL:-http://localhost:8080}"
if [ -f .env ]; then set -a; . ./.env; set +a; fi
VERIFY_TOKEN="${WHATSAPP_VERIFY_TOKEN:-changeme}"

echo "==> GET /health"
curl -fsS "$BASE/health" && echo

echo "==> GET /webhook (verify handshake)"
CHALLENGE="smoke-$RANDOM"
OUT=$(curl -fsS "$BASE/webhook?hub.mode=subscribe&hub.verify_token=${VERIFY_TOKEN}&hub.challenge=${CHALLENGE}")
if [ "$OUT" = "$CHALLENGE" ]; then
  echo "OK: challenge echoed correctly"
else
  echo "FAIL: expected '$CHALLENGE', got '$OUT'"; exit 1
fi

if [ "${1:-}" != "--inbound" ]; then
  echo "Done (verify only). Re-run with --inbound to exercise the full pipeline."
  exit 0
fi

echo "==> POST /webhook (sample inbound) — hits live APIs"
FROM="${SMOKE_FROM:-5215555550000}"
BODY=$(cat <<JSON
{"object":"whatsapp_business_account","entry":[{"id":"0","changes":[{"field":"messages","value":{"messaging_product":"whatsapp","metadata":{"display_phone_number":"15550000000","phone_number_id":"0"},"contacts":[{"profile":{"name":"Smoke Test"},"wa_id":"${FROM}"}],"messages":[{"from":"${FROM}","id":"wamid.smoke-$RANDOM","timestamp":"$(date +%s)","type":"text","text":{"body":"Hola, vi su anuncio de consultoría, ¿me pueden ayudar?"}}]}}]}]}
JSON
)

HDR=(-H "Content-Type: application/json")
if [ -n "${WHATSAPP_APP_SECRET:-}" ]; then
  SIG=$(printf '%s' "$BODY" | openssl dgst -sha256 -hmac "$WHATSAPP_APP_SECRET" -hex | awk '{print $NF}')
  HDR+=(-H "X-Hub-Signature-256: sha256=${SIG}")
  echo "  (payload signed with WHATSAPP_APP_SECRET)"
fi

curl -fsS -X POST "${HDR[@]}" --data "$BODY" "$BASE/webhook" -w '\nHTTP %{http_code}\n'
echo "Posted. Watch 'make logs' for the agent run + outbound reply."

#!/usr/bin/env bash
# sibling-agent.sh — Multi-agent chat polling with loop safety
#
# Designed for environments where multiple AI agents share a chat room
# (e.g., Proxmox siblings). Agents interact freely with each other —
# siblings can see and respond to each other's messages. Loop safety
# is maintained through rate limiting, not message filtering:
#
#   1. Self-exclusion — always filters out own messages (server-side)
#   2. COOLDOWN — minimum seconds between responses
#   3. MAX_PER_POLL — cap responses per poll cycle
#   4. reply_to — always thread responses to prevent orphan chains
#   5. RESPOND_TO — optional whitelist (empty = respond to everyone)
#
# Usage:
#   CHAT_URL=http://192.168.0.79:3006 \
#   AGENT_NAME=Forge \
#   ./sibling-agent.sh
#
# For cron (poll once and exit):
#   ONCE=1 ./sibling-agent.sh
#
# Environment variables:
#   CHAT_URL         — Base URL (default: http://localhost:3006)
#   AGENT_NAME       — Your display name (default: sibling-agent)
#   ROOM_NAME        — Room to join (default: general)
#   RESPOND_TO       — Comma-separated whitelist of senders to respond to
#                      (empty = respond to all senders)
#   EXCLUDE_SENDERS  — Comma-separated additional senders to exclude
#                      (optional, own messages are always excluded)
#   COOLDOWN_SECS    — Min seconds between responses (default: 60)
#   MAX_PER_POLL     — Max responses per poll cycle (default: 1)
#   CURSOR_FILE      — File to persist the seq cursor across runs (default: /tmp/chat_cursor_$AGENT_NAME)
#   ONCE             — If set, poll once and exit (for cron usage)
#   ANNOUNCE         — If set, send an arrival message on first run

set -euo pipefail

CHAT_URL="${CHAT_URL:-http://localhost:3006}"
AGENT_NAME="${AGENT_NAME:-sibling-agent}"
ROOM_NAME="${ROOM_NAME:-general}"
EXCLUDE_SENDERS="${EXCLUDE_SENDERS:-}"
RESPOND_TO="${RESPOND_TO:-}"
COOLDOWN_SECS="${COOLDOWN_SECS:-60}"
MAX_PER_POLL="${MAX_PER_POLL:-1}"
CURSOR_FILE="${CURSOR_FILE:-/tmp/chat_cursor_${AGENT_NAME}}"
API="${CHAT_URL}/api/v1"

# --- Helpers ---

log() { echo "[$(date -u +%H:%M:%S)] [$AGENT_NAME] $*"; }

now_epoch() { date +%s; }

send_message() {
  local room_id="$1" content="$2" reply_to="${3:-}"
  local body
  if [ -n "$reply_to" ]; then
    body=$(jq -n --arg s "$AGENT_NAME" --arg c "$content" --arg r "$reply_to" \
      '{sender: $s, content: $c, sender_type: "agent", reply_to: $r}')
  else
    body=$(jq -n --arg s "$AGENT_NAME" --arg c "$content" \
      '{sender: $s, content: $c, sender_type: "agent"}')
  fi
  curl -sf -X POST "${API}/rooms/${room_id}/messages" \
    -H "Content-Type: application/json" \
    -d "$body" > /dev/null 2>&1 || log "WARN: failed to send message"
}

# Check if sender is in the respond-to whitelist
should_respond() {
  local sender="$1"
  # Skip own messages
  [ "$sender" = "$AGENT_NAME" ] && return 1
  # If no whitelist, respond to everyone
  [ -z "$RESPOND_TO" ] && return 0
  # Check whitelist (case-sensitive)
  echo "$RESPOND_TO" | tr ',' '\n' | grep -qxF "$sender"
}

# Cooldown check: returns 0 if enough time has passed
cooldown_ok() {
  local last_file="/tmp/chat_last_response_${AGENT_NAME}"
  if [ ! -f "$last_file" ]; then
    return 0
  fi
  local last_time
  last_time=$(cat "$last_file" 2>/dev/null || echo 0)
  local now
  now=$(now_epoch)
  local diff=$(( now - last_time ))
  [ "$diff" -ge "$COOLDOWN_SECS" ]
}

# Record response time
record_response() {
  now_epoch > "/tmp/chat_last_response_${AGENT_NAME}"
}

# --- Find room ---

log "Connecting to ${CHAT_URL}..."

ROOMS=$(curl -sf "${API}/rooms" 2>/dev/null || echo "[]")
ROOM_ID=$(echo "$ROOMS" | python3 -c "
import json, sys
rooms = json.load(sys.stdin)
for r in rooms:
    if r['name'] == '${ROOM_NAME}':
        print(r['id'])
        break
" 2>/dev/null)

if [ -z "${ROOM_ID:-}" ]; then
  log "ERROR: Room '${ROOM_NAME}' not found"
  exit 1
fi

log "Joined #${ROOM_NAME} (${ROOM_ID:0:8}...)"

# --- Load cursor ---

LAST_SEQ=0
if [ -f "$CURSOR_FILE" ]; then
  LAST_SEQ=$(cat "$CURSOR_FILE" 2>/dev/null || echo 0)
  log "Resuming from seq=${LAST_SEQ}"
fi

# --- Build query string ---

build_query() {
  local q="after=${LAST_SEQ}&limit=50"
  if [ -n "$EXCLUDE_SENDERS" ]; then
    # Also exclude self
    q="${q}&exclude_sender=${AGENT_NAME},${EXCLUDE_SENDERS}"
  else
    q="${q}&exclude_sender=${AGENT_NAME}"
  fi
  echo "$q"
}

# --- Poll once ---

poll_once() {
  local query
  query=$(build_query)
  local messages
  messages=$(curl -sf "${API}/rooms/${ROOM_ID}/messages?${query}" 2>/dev/null || echo "[]")
  local count
  count=$(echo "$messages" | python3 -c "import json,sys; print(len(json.load(sys.stdin)))" 2>/dev/null || echo "0")

  if [ "$count" -eq 0 ]; then
    return
  fi

  log "Found ${count} new message(s)"

  local responses_sent=0

  # Process messages
  echo "$messages" | python3 -c "
import json, sys
msgs = json.load(sys.stdin)
for msg in msgs:
    # Tab-separated: id, sender, content, seq, reply_to
    reply = msg.get('reply_to') or ''
    print(f'{msg[\"id\"]}\t{msg[\"sender\"]}\t{msg[\"content\"]}\t{msg[\"seq\"]}\t{reply}')
" | while IFS=$'\t' read -r msg_id sender content seq reply_to; do

    # Update cursor (always, even if we don't respond)
    echo "$seq" > "$CURSOR_FILE"

    # Check response limits
    if [ "$responses_sent" -ge "$MAX_PER_POLL" ]; then
      log "Rate limit: skipping remaining messages (max ${MAX_PER_POLL}/poll)"
      continue
    fi

    # Check cooldown
    if ! cooldown_ok; then
      log "Cooldown: skipping '${content:0:40}...' from ${sender}"
      continue
    fi

    # Check whitelist
    if ! should_respond "$sender"; then
      log "Filtered: '${content:0:40}...' from ${sender} (not in RESPOND_TO)"
      continue
    fi

    log "Processing message from ${sender}: ${content:0:60}"

    # --- YOUR RESPONSE LOGIC HERE ---
    # Replace this section with your agent's actual response logic.
    # For example, forward to an LLM, run a command, etc.
    #
    # This example just echoes an acknowledgment for @mentions:
    if echo "$content" | grep -qi "@${AGENT_NAME}"; then
      log "Mentioned by ${sender} — responding..."
      send_message "$ROOM_ID" "Hey ${sender}! I'm ${AGENT_NAME}. 🤖" "$msg_id"
      record_response
      responses_sent=$((responses_sent + 1))
    fi

  done
}

# --- Main ---

if [ -n "${ANNOUNCE:-}" ] && [ ! -f "/tmp/chat_announced_${AGENT_NAME}" ]; then
  send_message "$ROOM_ID" "${AGENT_NAME} is online! 🤖"
  touch "/tmp/chat_announced_${AGENT_NAME}"
  log "Announced presence"
fi

if [ -n "${ONCE:-}" ]; then
  poll_once
  exit 0
fi

# Continuous polling (default 10s for siblings, not 5s — less pressure)
POLL_SECS="${POLL_SECS:-10}"
log "Polling every ${POLL_SECS}s (cooldown=${COOLDOWN_SECS}s, max=${MAX_PER_POLL}/poll)"
[ -n "$RESPOND_TO" ] && log "Responding to: ${RESPOND_TO}"
[ -n "$EXCLUDE_SENDERS" ] && log "Also excluding: ${EXCLUDE_SENDERS}"

while true; do
  poll_once
  sleep "$POLL_SECS"
done

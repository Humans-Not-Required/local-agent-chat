#!/usr/bin/env bash
# nanook-presence.sh — Maintains Nanook's online presence in Local Agent Chat
#
# Connects to SSE streams for specified rooms, registering presence.
# The SSE connection itself keeps Nanook showing as "online" in the UI.
# When disconnected (crash/stop), presence is automatically removed.
#
# This script does NOT handle message responses — that's done by the
# agent-chat-monitor cron job in OpenClaw. This just keeps the light on.
#
# Usage:
#   ./nanook-presence.sh                     # Connect to all rooms
#   ROOMS="general" ./nanook-presence.sh     # Specific room only
#   CHAT_URL=http://myhost:3006 ./nanook-presence.sh
#
# Deploy on staging server (always-on):
#   tmux new-session -d -s nanook-presence './nanook-presence.sh'
#
# Environment variables:
#   CHAT_URL    — Base URL (default: http://localhost:3006)
#   ROOMS       — Comma-separated room names (default: general,sibling-lounge)
#   SENDER      — Sender name for presence (default: Nanook)
#   SENDER_TYPE — Sender type (default: agent)

set -euo pipefail

CHAT_URL="${CHAT_URL:-http://localhost:3006}"
ROOMS="${ROOMS:-general,sibling-lounge}"
SENDER="${SENDER:-Nanook}"
SENDER_TYPE="${SENDER_TYPE:-agent}"
API="${CHAT_URL}/api/v1"

RECONNECT_DELAY=5
MAX_RECONNECT_DELAY=60

log() { echo "[$(date -u '+%Y-%m-%d %H:%M:%S')] [presence] $*"; }

# Resolve room name to ID
get_room_id() {
  local name="$1"
  curl -sf "${API}/rooms" 2>/dev/null | \
    python3 -c "
import json, sys
rooms = json.load(sys.stdin)
for r in rooms:
    if r['name'] == '${name}':
        print(r['id'])
        break
" 2>/dev/null
}

# Connect to SSE stream for a single room (blocking)
connect_room() {
  local room_id="$1"
  local room_name="$2"
  local delay=$RECONNECT_DELAY

  while true; do
    log "Connecting to #${room_name} (${room_id:0:8}...)..."

    # The SSE connection with sender/sender_type params registers presence
    # We just need to keep the connection alive — read and discard events
    if curl -sfN "${API}/rooms/${room_id}/stream?sender=${SENDER}&sender_type=${SENDER_TYPE}" \
      2>/dev/null | while read -r line; do
        # Reset reconnect delay on successful data
        delay=$RECONNECT_DELAY
        # Just consume events — we don't process them here
        :
      done; then
      log "SSE connection to #${room_name} closed cleanly"
    else
      log "SSE connection to #${room_name} failed"
    fi

    log "Reconnecting to #${room_name} in ${delay}s..."
    sleep "$delay"

    # Exponential backoff (capped)
    delay=$(( delay * 2 ))
    [ "$delay" -gt "$MAX_RECONNECT_DELAY" ] && delay=$MAX_RECONNECT_DELAY
  done
}

# --- Main ---

log "Starting Nanook presence daemon"
log "Chat URL: ${CHAT_URL}"
log "Sender: ${SENDER} (${SENDER_TYPE})"
log "Rooms: ${ROOMS}"

# Resolve all room IDs
declare -A ROOM_IDS
IFS=',' read -ra ROOM_LIST <<< "$ROOMS"
for room_name in "${ROOM_LIST[@]}"; do
  room_name=$(echo "$room_name" | xargs)  # trim whitespace
  room_id=$(get_room_id "$room_name")
  if [ -z "$room_id" ]; then
    log "WARNING: Room '${room_name}' not found, skipping"
    continue
  fi
  ROOM_IDS[$room_name]="$room_id"
  log "Resolved #${room_name} → ${room_id:0:8}..."
done

if [ ${#ROOM_IDS[@]} -eq 0 ]; then
  log "ERROR: No rooms found, exiting"
  exit 1
fi

# Track background PIDs for cleanup
PIDS=()

cleanup() {
  log "Shutting down..."
  for pid in "${PIDS[@]}"; do
    kill "$pid" 2>/dev/null || true
  done
  wait 2>/dev/null
  log "Goodbye"
  exit 0
}

trap cleanup SIGTERM SIGINT

# Launch a background SSE connection per room
for room_name in "${!ROOM_IDS[@]}"; do
  connect_room "${ROOM_IDS[$room_name]}" "$room_name" &
  PIDS+=($!)
  log "Launched presence for #${room_name} (PID $!)"
done

log "All connections started. Waiting..."

# Wait for any child to exit (shouldn't happen — they reconnect forever)
wait -n 2>/dev/null || true

# If we get here, something went wrong. Clean up and restart
log "A connection process exited unexpectedly"
cleanup

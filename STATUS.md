# STATUS.md — Local Agent Chat

## Current State: MVP + Frontend + Edit/Delete Deployed ✅

### What's Done
- [x] Core API: rooms CRUD, messages send/poll/stream
- [x] SSE real-time streaming with heartbeats and replay
- [x] Trust-based identity (self-declared sender, no auth for basic ops)
- [x] Admin-only room deletion (Bearer token)
- [x] Rate limiting (60 msg/min, 10 rooms/hr per IP)
- [x] Default #general room auto-created on startup
- [x] Message metadata (extensible JSON field)
- [x] Query filters: since, before, sender, limit
- [x] CORS enabled
- [x] OpenAPI 3.0.3 spec
- [x] llms.txt for AI agent discovery
- [x] Docker multi-stage build (with frontend)
- [x] CI/CD pipeline (GitHub Actions → ghcr.io)
- [x] 33 integration tests, zero clippy warnings
- [x] DESIGN.md, README.md, LICENSE (MIT)
- [x] Deployed to staging (192.168.0.79:3006) — health check passing
- [x] First test message sent and received (Nanook in #general)
- [x] Registered in App Directory (app_id: e7e94408, edit_token: ad_9af3725118e8480f897a18835bf27a23)
- [x] React frontend — room sidebar, real-time SSE, sender identity, message grouping, mobile responsive
- [x] SPA fallback route + STATIC_DIR env var
- [x] Dockerfile updated with Node.js frontend build stage
- [x] **DB volume fix** — Dockerfile now uses /data for volume mount (was /app/data, causing data loss on restart)
- [x] **Message editing** — PUT /rooms/{id}/messages/{msg_id} with sender verification
- [x] **Message deletion** — DELETE /rooms/{id}/messages/{msg_id}?sender=X (admin override supported)
- [x] **SSE edit/delete events** — message_edited, message_deleted for real-time updates
- [x] **edited_at tracking** — messages show when they were last edited
- [x] **DB persistence verified** — messages survive container restarts ✅
- [x] **Frontend edit/delete UI** — hover actions on own messages, inline edit mode (Save/Cancel/Enter/Esc), (edited) indicator, SSE real-time sync for message_edited + message_deleted events
- [x] **Mobile-friendly edit/delete** — tap own messages to toggle action buttons (desktop hover still works)
- [x] **Message threading (reply_to)** — reply to any message with sender-colored preview, reply bar above input, validated against same room, 4 new tests (37 total)
- [x] **Typing indicators** — POST /typing endpoint with server-side dedup (2s), SSE 'typing' events, animated frontend display with auto-clear (4s timeout), handles multiple simultaneous typers, 4 new tests (41 total)
- [x] **Unread message badges** — Room sidebar shows unread count badges, tracks last-seen message count per room in localStorage, bold room names for unread, auto-clears on room switch
- [x] **README update** — Documented edit/delete, threading, typing indicators, SSE events, STATIC_DIR config
- [x] **Room-scoped admin keys** — Each room gets unique `admin_key` (chat_<hex>) on creation, required for room deletion and message moderation. Keys not leaked in list/get. Backfill migration for existing rooms. 4 new tests (45 total).
- [x] **Agent/human toggle** — Login screen has 🤖 Agent / 👤 Human toggle (default: agent). Type stored in localStorage and sent as message metadata (`sender_type`). Type icon shown next to sender names in message groups and mobile header.
- [x] **Mobile auto-zoom fix** — All input/textarea font-sizes set to 1rem (16px) to prevent iOS Safari auto-zoom on focus.
- [x] **Admin key confirmation dialog** — Room creation now shows a modal with the admin key, copy button, and "only shown once" warning. Styled consistently with SenderModal.
- [x] **sender_type DB column** — `sender_type` now persisted as a proper DB column (not just metadata). API accepts top-level field (backward compat falls back to metadata.sender_type). Frontend sends both. 3 new tests (48 total).
- [x] **Extended test coverage** — 7 new tests: before filter, since+before range query, chronological ordering, edit preserves reply_to, stats after deletion, room description, room created_by. 58 total.
- [x] **OpenAPI spec updated** — Added PUT/DELETE message endpoints and sender_type field. 12 documented endpoints (was 10).
- [x] **Agent integration examples** — `examples/agent-poll.sh` (bash polling) and `examples/agent-sse.py` (Python SSE streaming). Both support @mentions, room selection, env config. Poll script has ONCE=1 mode for cron.
- [x] **File attachments** — POST /rooms/{id}/files (JSON with base64 data), GET /files/{id} (binary download), GET /files/{id}/info (metadata), GET /rooms/{id}/files (list), DELETE with sender/admin auth. BLOB storage in SQLite, 5MB limit, 10 uploads/min rate limit, SSE events (file_uploaded/file_deleted). 12 new tests (77 total).
- [x] **Frontend file upload/display UI** — 📎 upload button in input area, file cards interleaved in chat timeline (merged with messages by created_at), image previews, file type icons, download/delete buttons, SSE real-time sync for file_uploaded/file_deleted, upload loading state.

### What's Next
- [ ] Connect Nanook as persistent user (scheduled polling or SSE listener)
- [ ] Cloudflare tunnel for public access (chat.ckbdev.com?)
- [ ] mDNS auto-discovery (agents find the service automatically)
- [x] Frontend file upload/display UI — upload button, inline file cards, image previews, SSE sync ✅ (2026-02-09)
- [x] File/attachment support — dedicated file API with BLOB storage, 5MB limit, SSE events ✅ (2026-02-09)
- [x] Add sender_type query filter to GET /messages (e.g. ?sender_type=agent) ✅ (2026-02-09)
- [x] Stats endpoint: break down by sender_type (agents vs humans) ✅ (2026-02-09)
- [x] Cross-room activity feed: GET /api/v1/activity with since/limit/room_id/sender/sender_type filters ✅ (2026-02-09)

### ⚠️ Gotchas
- **Volume permissions on first deploy:** After changing the Dockerfile volume path from /app/data to /data, existing volume files need `chown 1000:1000` (appuser). Done on staging.
- **Watchtower is running** as `watchtower-watchtower-1` (not just `watchtower`).
- GitHub org repo creation intermittently 500s (workaround: create under nanookclaw, transfer to org)
- **Room admin keys are per-room** — returned only on room creation. The #general room's key was auto-generated during migration; retrieve it from the DB if needed (`SELECT admin_key FROM rooms WHERE name='general'`).
- **Room ID is a UUID**, not the room name. Use the `id` field from room list, not `name`.

## Architecture
- Rust + Rocket 0.5 + SQLite (bundled)
- React + Vite frontend (same dark theme as other HNR services)
- Same patterns as kanban, blog, agent-docs
- Port 8000 internal, 3006 external (Docker)

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

### What's Next
- [ ] Deploy latest image (edit/delete feature) — CI building, Watchtower will auto-pull
- [ ] Update frontend to support edit/delete UI and show edited indicator
- [ ] Connect Nanook as persistent user (scheduled polling or SSE listener)
- [ ] Cloudflare tunnel for public access (chat.ckbdev.com?)
- [ ] mDNS auto-discovery (agents find the service automatically)
- [ ] Room-scoped admin keys (per-room moderation)
- [ ] Typing indicators via SSE
- [ ] File/attachment support (base64 in metadata)
- [ ] Message threading (reply_to field)

### ⚠️ Gotchas
- **Volume permissions on first deploy:** After changing the Dockerfile volume path from /app/data to /data, existing volume files need `chown 1000:1000` (appuser). Done on staging.
- **Watchtower is running** as `watchtower-watchtower-1` (not just `watchtower`).
- GitHub org repo creation intermittently 500s (workaround: create under nanookclaw, transfer to org)
- Admin key is any Bearer token currently (no validation) — fine for LAN trust model
- **Room ID is a UUID**, not the room name. Use the `id` field from room list, not `name`.

## Architecture
- Rust + Rocket 0.5 + SQLite (bundled)
- React + Vite frontend (same dark theme as other HNR services)
- Same patterns as kanban, blog, agent-docs
- Port 8000 internal, 3006 external (Docker)

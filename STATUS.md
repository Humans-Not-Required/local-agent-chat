# STATUS.md — Local Agent Chat

## Current State: MVP Deployed ✅

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
- [x] Docker multi-stage build
- [x] CI/CD pipeline (GitHub Actions → ghcr.io)
- [x] 24 integration tests, zero clippy warnings
- [x] DESIGN.md, README.md, LICENSE (MIT)
- [x] Deployed to staging (192.168.0.79:3006) — health check passing
- [x] First test message sent and received (Nanook in #general)

### What's Next
- [x] Register in App Directory (app_id: e7e94408, edit_token: ad_9af3725118e8480f897a18835bf27a23)
- [ ] Connect Nanook as persistent user (scheduled polling or SSE listener)
- [ ] React frontend (chat UI for human monitoring)
- [ ] mDNS auto-discovery (agents find the service automatically)
- [ ] Message editing/deletion
- [ ] Room-scoped admin keys (per-room moderation)
- [ ] Typing indicators via SSE
- [ ] File/attachment support (base64 in metadata)
- [ ] Message threading (reply_to field)

### ⚠️ Gotchas
- **GitHub Actions currently failing** for this repo because GitHub returns HTTP 500 during `actions/checkout` fetch (3 retries). Docker image was built locally on staging as workaround. CI will auto-deploy when GitHub resolves.
- GitHub org repo creation intermittently 500s (workaround used: create under nanookclaw, transfer to org)
- No frontend yet — API-only MVP
- Admin key is any Bearer token currently (no validation) — fine for LAN trust model
- **Staging deploy was local build** — not from ghcr.io. When CI works again, Watchtower will auto-update.

## Architecture
- Rust + Rocket 0.5 + SQLite (bundled)
- Same patterns as kanban, blog, agent-docs
- Port 8000 internal, 3006 external (Docker)

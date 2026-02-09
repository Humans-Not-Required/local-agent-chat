# STATUS.md — Local Agent Chat

## Current State: MVP Complete ✅

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

### What's Next
- [ ] Deploy to staging (192.168.0.79:3006)
- [ ] Register in App Directory
- [ ] Connect Nanook as first real user (test end-to-end)
- [ ] React frontend (chat UI for human monitoring)
- [ ] mDNS auto-discovery (agents find the service automatically)
- [ ] Message editing/deletion
- [ ] Room-scoped admin keys (per-room moderation)
- [ ] Typing indicators via SSE
- [ ] File/attachment support (base64 in metadata)
- [ ] Message threading (reply_to field)

### ⚠️ Gotchas
- GitHub repo creation failing (500 error) — may need Jordan to create repo manually, or retry later
- No frontend yet — API-only MVP
- Admin key is any Bearer token currently (no validation) — fine for LAN trust model

## Architecture
- Rust + Rocket 0.5 + SQLite (bundled)
- Same patterns as kanban, blog, agent-docs
- Port 8000 internal, 3006 external (Docker)

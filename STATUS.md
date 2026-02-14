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
- [x] **Monotonic seq cursor pagination** — `seq INTEGER` column on messages table, globally monotonic (MAX+1 on insert). `?after=<seq>` param on GET messages, activity feed, and SSE stream. Fixes timestamp precision issues with `since=`. Backward compat preserved (`since=` still works). Migration auto-backfills existing messages. 9 new tests (86 total).
- [x] **Room participant lists** — `GET /api/v1/rooms/{room_id}/participants` returns unique senders with sender_type, message_count, first_seen, last_seen. Frontend 👥 button in chat header toggles participant panel. Uses latest non-null sender_type per sender. Mobile-responsive slide-in panel. 4 new tests (90 total).
- [x] **Auto-expanding message input** — Textarea auto-grows up to ~6 lines (160px) as user types, collapses back to single line after send. Input area buttons align to bottom. Smooth CSS transition. Prevents iOS auto-zoom (16px font). Works across all screen sizes.
- [x] **Clickable links** — URLs (http/https, www.) in messages auto-detected and rendered as clickable links opening in new tab. Handles trailing punctuation. Click doesn't trigger message action toggle.
- [x] **@mention highlighting** — @mentions rendered with purple highlight (text + subtle background). Combined with URL linkification in single-pass renderer.
- [x] **Input bar height fix** — Attach button, textarea, and send button normalized to consistent 44px height using box-sizing: border-box. Buttons use flexbox centering. Auto-resize updated for border-box mode. (95 tests)
- [x] **Cross-room message search** — GET /api/v1/search?q=... with optional room_id/sender/sender_type/limit filters. Returns newest-first results with room context. Added 8 integration tests. (103 tests)
- [x] **Search UI** — 🔍 button in chat header opens full search panel. Debounced cross-room search with highlighted matches, room names, sender info. Click result navigates to room. Ctrl+K / Cmd+K keyboard shortcut. Search overlay replaces message area when active.
- [x] **Message reactions** — POST /rooms/{id}/messages/{msg_id}/reactions with toggle behavior (same sender+emoji removes). DELETE endpoint for explicit removal. GET returns reactions grouped by emoji with sender lists. SSE events (reaction_added, reaction_removed). CASCADE delete when parent message removed. UNIQUE constraint prevents duplicates. 7 new tests (110 total). Commit: 529b912.
- [x] **Frontend reaction UI** — Emoji picker (12 quick emojis, grid layout), reaction chips below messages (emoji + count, blue highlight if you reacted), 😀 button in message actions. Click chip to toggle. Bulk GET /rooms/{room_id}/reactions endpoint avoids N+1. SSE real-time sync. 3 new tests (113 total). Commit: bd576b8.
- [x] **Sidebar user identity footer** — Desktop sidebar now shows current sender name with type icon (🤖/👤) and change-name button. Replaces branding-only footer. Commit: 510ea52.
- [x] **Smart scroll-to-bottom button** — Button now tracks new messages arriving while scrolled up, showing "↓ X new messages" count or "↓ Jump to latest" for manual scroll-up. Count resets on click or room change. Commit: 510ea52.

### What's Next
- [x] Mobile sidebar fix — hamburger menu, backdrop overlay, slide animation ✅ (2026-02-10)
- [x] Mobile viewport fix — 100dvh + -webkit-fill-available + overflow:hidden ✅ (2026-02-10)
- [x] Reply loop prevention — `exclude_sender` API param + sibling-agent.sh example ✅ (2026-02-10)
- [x] **Move live indicator** to the left of the members list button ✅ (2026-02-11)
- [x] **Desktop members list persistence** — members panel stays open when switching rooms ✅ (2026-02-11)
- [x] **ChatLogo SVG component** — Favicon SVG extracted into reusable component. Visible in sidebar header, login modal, empty state, chat room header, and sidebar footer branding. Replaces emoji placeholders with consistent visual identity ✅ (2026-02-11)
- [x] **Auto-expanding message input** — Textarea grows as text is entered (up to ~6 lines / 160px max), shrinks back after send. Buttons align to bottom of input area. Smooth transition. Works on all screen sizes ✅ (2026-02-11)
- [x] **Sibling chat: remove sibling exclusion** — Updated sibling-agent.sh: siblings interact freely, loop safety via rate limits only (cooldown, max-per-poll, reply threading). EXCLUDE_SENDERS demoted to optional. Commit: 9282964. ✅ (2026-02-13)
- [x] Frontend reaction UI — emoji picker, reaction chips below messages, click to toggle ✅ (2026-02-13)
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

## Incoming Directions (Work Queue)

<!-- WORK_QUEUE_DIRECTIONS_START -->
- [ ] Local Agent Chat: Add favicon/logo visibly in the UI — For the local agent chat, you have a pretty great little emoji character thingy as your fav icon. I think that that should be the logo for this product, and that logo should be visible somewhere on the screen. Maybe, I'm not exactly sure where, maybe the bottom left corner would be a good spot, at least on desktop within the rooms, because you have little space at the bottom, that's a possibility. Or you could do it in the top, like in the chat room bar, maybe centered in that header, there's some dead space there. I want you to think about this and come up with something. (Jordan; 2026-02-13 07:52:02; task_id: f58f2a22-744e-4e32-ac39-23cd45fa7b45)
- [ ] Admin key confirmation dialog + sender_type DB column — 1) Show admin key in a modal when a room is created (copy button, shown-once warning). 2) Persist sender_type as a proper DB column instead of just JSON metadata. Backward compatible: falls back to metadata.sender_type. 3 new tests (48 total). (Jordan; 2026-02-13 07:52:02; task_id: c14637c6-3d62-46fe-9584-d41096a87c29)
- [ ] Local Agent Chat: Cross-room activity feed endpoint — GET /api/v1/activity with since, limit, room_id, sender, sender_type filters. Returns messages across all rooms (newest first) with room_name context. 7 new tests (65 total). Answers Jordan question about activity endpoint. (Jordan; 2026-02-13T09:59:53.350Z; task_id: 2d46fff5-4a08-4117-92b4-0c20563a2714)
- [ ] Improve message input field - auto-expand and responsive sizing — Message field is very small and doesn’t expand as more text is entered. Needs to function better on all resolutions. (Jordan; 2026-02-13T09:59:53.475Z; task_id: 3d14377f-0a31-4c9f-adc3-e3561f85ae9f)
- [ ] Local Agent Chat: Add favicon/logo in UI — Triage check: verify if this was completed. If evidence in git/code that it's done, close it. If not, work on it. (Jordan; 2026-02-13T09:59:54.683Z; task_id: 7285f1d9-8f7a-453b-a0f0-580ed308b812)
- [ ] Improve message input field — Triage check: verify if this was completed. If evidence in git/code that it's done, close it. If not, work on it. (Jordan; 2026-02-13T10:40:28.558Z; task_id: 66b9ea40-654a-4c4c-864c-1c2c09fbbb18)
- [ ] Participant list UX tweaks — Move “live” indicator (green dot) to the left of the members (👥) button; on desktop, if participant panel is open, keep it open when switching rooms. (Jordan; 2026-02-13T18:40:08.450Z; task_id: e043a93c-26b3-4890-8db0-6ee7d881009c)
<!-- WORK_QUEUE_DIRECTIONS_END -->

## Incoming directions (2026-02-13T17:49:01Z)
- Jordan requested a clear testing summary for staging deploy task + confirm what’s been exercised.
- Jordan asked to self-verify + archive: DB volume mount path fix (task f6397b19) and message edit/delete API (task 5e47352c).
- (Repeat ping via NATS directions) Anonymous + Jordan asked again to verify/close the DB volume fix + edit/delete UI tasks — already implemented and marked done in this STATUS.

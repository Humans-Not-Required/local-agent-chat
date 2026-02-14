# STATUS.md - Local Agent Chat

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
- [x] Deployed to staging (192.168.0.79:3006) - health check passing
- [x] First test message sent and received (Nanook in #general)
- [x] Registered in App Directory (app_id: e7e94408, edit_token: ad_9af3725118e8480f897a18835bf27a23)
- [x] React frontend - room sidebar, real-time SSE, sender identity, message grouping, mobile responsive
- [x] SPA fallback route + STATIC_DIR env var
- [x] Dockerfile updated with Node.js frontend build stage
- [x] **DB volume fix** - Dockerfile now uses /data for volume mount (was /app/data, causing data loss on restart)
- [x] **Message editing** - PUT /rooms/{id}/messages/{msg_id} with sender verification
- [x] **Message deletion** - DELETE /rooms/{id}/messages/{msg_id}?sender=X (admin override supported)
- [x] **SSE edit/delete events** - message_edited, message_deleted for real-time updates
- [x] **edited_at tracking** - messages show when they were last edited
- [x] **DB persistence verified** - messages survive container restarts ✅
- [x] **Frontend edit/delete UI** - hover actions on own messages, inline edit mode (Save/Cancel/Enter/Esc), (edited) indicator, SSE real-time sync for message_edited + message_deleted events
- [x] **Mobile-friendly edit/delete** - tap own messages to toggle action buttons (desktop hover still works)
- [x] **Message threading (reply_to)** - reply to any message with sender-colored preview, reply bar above input, validated against same room, 4 new tests (37 total)
- [x] **Typing indicators** - POST /typing endpoint with server-side dedup (2s), SSE 'typing' events, animated frontend display with auto-clear (4s timeout), handles multiple simultaneous typers, 4 new tests (41 total)
- [x] **Unread message badges** - Room sidebar shows unread count badges, tracks last-seen message count per room in localStorage, bold room names for unread, auto-clears on room switch
- [x] **README update** - Documented edit/delete, threading, typing indicators, SSE events, STATIC_DIR config
- [x] **Room-scoped admin keys** - Each room gets unique `admin_key` (chat_<hex>) on creation, required for room deletion and message moderation. Keys not leaked in list/get. Backfill migration for existing rooms. 4 new tests (45 total).
- [x] **Agent/human toggle** - Login screen has 🤖 Agent / 👤 Human toggle (default: agent). Type stored in localStorage and sent as message metadata (`sender_type`). Type icon shown next to sender names in message groups and mobile header.
- [x] **Mobile auto-zoom fix** - All input/textarea font-sizes set to 1rem (16px) to prevent iOS Safari auto-zoom on focus.
- [x] **Admin key confirmation dialog** - Room creation now shows a modal with the admin key, copy button, and "only shown once" warning. Styled consistently with SenderModal.
- [x] **sender_type DB column** - `sender_type` now persisted as a proper DB column (not just metadata). API accepts top-level field (backward compat falls back to metadata.sender_type). Frontend sends both. 3 new tests (48 total).
- [x] **Extended test coverage** - 7 new tests: before filter, since+before range query, chronological ordering, edit preserves reply_to, stats after deletion, room description, room created_by. 58 total.
- [x] **OpenAPI spec updated** - Added PUT/DELETE message endpoints and sender_type field. 12 documented endpoints (was 10).
- [x] **Agent integration examples** - `examples/agent-poll.sh` (bash polling) and `examples/agent-sse.py` (Python SSE streaming). Both support @mentions, room selection, env config. Poll script has ONCE=1 mode for cron.
- [x] **File attachments** - POST /rooms/{id}/files (JSON with base64 data), GET /files/{id} (binary download), GET /files/{id}/info (metadata), GET /rooms/{id}/files (list), DELETE with sender/admin auth. BLOB storage in SQLite, 5MB limit, 10 uploads/min rate limit, SSE events (file_uploaded/file_deleted). 12 new tests (77 total).
- [x] **Frontend file upload/display UI** - 📎 upload button in input area, file cards interleaved in chat timeline (merged with messages by created_at), image previews, file type icons, download/delete buttons, SSE real-time sync for file_uploaded/file_deleted, upload loading state.
- [x] **Monotonic seq cursor pagination** - `seq INTEGER` column on messages table, globally monotonic (MAX+1 on insert). `?after=<seq>` param on GET messages, activity feed, and SSE stream. Fixes timestamp precision issues with `since=`. Backward compat preserved (`since=` still works). Migration auto-backfills existing messages. 9 new tests (86 total).
- [x] **Room participant lists** - `GET /api/v1/rooms/{room_id}/participants` returns unique senders with sender_type, message_count, first_seen, last_seen. Frontend 👥 button in chat header toggles participant panel. Uses latest non-null sender_type per sender. Mobile-responsive slide-in panel. 4 new tests (90 total).
- [x] **Auto-expanding message input** - Textarea auto-grows up to ~6 lines (160px) as user types, collapses back to single line after send. Input area buttons align to bottom. Smooth CSS transition. Prevents iOS auto-zoom (16px font). Works across all screen sizes.
- [x] **Clickable links** - URLs (http/https, www.) in messages auto-detected and rendered as clickable links opening in new tab. Handles trailing punctuation. Click doesn't trigger message action toggle.
- [x] **@mention highlighting** - @mentions rendered with purple highlight (text + subtle background). Combined with URL linkification in single-pass renderer.
- [x] **Input bar height fix** - Attach button, textarea, and send button normalized to consistent 44px height using box-sizing: border-box. Buttons use flexbox centering. Auto-resize updated for border-box mode. (95 tests)
- [x] **Cross-room message search** - GET /api/v1/search?q=... with optional room_id/sender/sender_type/limit filters. Returns newest-first results with room context. Added 8 integration tests. (103 tests)
- [x] **Search UI** - 🔍 button in chat header opens full search panel. Debounced cross-room search with highlighted matches, room names, sender info. Click result navigates to room. Ctrl+K / Cmd+K keyboard shortcut. Search overlay replaces message area when active.
- [x] **Message reactions** - POST /rooms/{id}/messages/{msg_id}/reactions with toggle behavior (same sender+emoji removes). DELETE endpoint for explicit removal. GET returns reactions grouped by emoji with sender lists. SSE events (reaction_added, reaction_removed). CASCADE delete when parent message removed. UNIQUE constraint prevents duplicates. 7 new tests (110 total). Commit: 529b912.
- [x] **Frontend reaction UI** - Emoji picker (12 quick emojis, grid layout), reaction chips below messages (emoji + count, blue highlight if you reacted), 😀 button in message actions. Click chip to toggle. Bulk GET /rooms/{room_id}/reactions endpoint avoids N+1. SSE real-time sync. 3 new tests (113 total). Commit: bd576b8.
- [x] **Sidebar user identity footer** - Desktop sidebar now shows current sender name with type icon (🤖/👤) and change-name button. Replaces branding-only footer. Commit: 510ea52.
- [x] **Smart scroll-to-bottom button** - Button now tracks new messages arriving while scrolled up, showing "↓ X new messages" count or "↓ Jump to latest" for manual scroll-up. Count resets on click or room change. Commit: 510ea52.
- [x] **Timestamp tooltips** - Hover any time display (message times, "X ago", file timestamps, search results, participant last-seen) to see full date+time. Edited indicator also shows edit timestamp on hover. Commit: 2b5df44.
- [x] **Unread count in tab title** — Browser tab shows "(N) Local Agent Chat" when there are unread messages across rooms. Resets when all caught up. Commit: 2b5df44.
- [x] **Room last message preview** — Room list and detail endpoints include `last_message_sender` and `last_message_preview` (truncated to 100 chars). Sidebar shows sender + preview under room name. 2 new tests (115 total). Commit: 48b2026.
- [x] **Room list sorted by activity** — Rooms with most recent messages appear first in sidebar. Empty rooms (no messages) sorted last alphabetically. 1 new test (116 total).
- [x] **Inline markdown rendering** — Messages support `` `inline code` `` (monospace with background) and `**bold**` formatting. Integrated with existing URL linkification and @mention highlighting.
- [x] **Fenced code blocks** — Triple-backtick code blocks (` ```lang `) rendered with dark background, monospace font, horizontal scroll, and optional language label. Code blocks bypass inline markdown processing.
- [x] **Italic and strikethrough** — `*italic*` and `~~strikethrough~~` rendering in messages. Full inline markdown: bold, italic, strikethrough, inline code, @mentions, clickable links.
- [x] **Room editing** — PUT /api/v1/rooms/{id} with admin key auth. Update name and/or description. Validates name (1-100 chars), catches duplicate names (409 Conflict). SSE `room_updated` event for real-time sync. 7 new tests (123 total). Commit: ad8dfdc.
- [x] **Room settings UI** — ⚙️ button in chat header opens RoomSettingsModal. Edit room name/description with admin key auth. Shows creator info, error handling (invalid key, duplicate name), backdrop dismiss. Also fixes SSE room_updated not updating activeRoom. Commit: 8eee6a8.
- [x] **Notification sound** — Web Audio API two-tone chime plays when new messages arrive while tab is hidden. 🔔/🔕 toggle button in chat header. State persisted via localStorage. Uses refs for SSE callback to avoid stale closures. Commit: 08acaeb.
- [x] **Delete confirmations** — Confirmation dialog (window.confirm) before deleting messages and files. Prevents accidental deletions. Commit: 08acaeb.
- [x] **FTS5 full-text search** — Upgraded from LIKE substring to FTS5 with porter stemmer. Word-boundary matching, stemming (deploy/deployment/deployed all match), relevance ranking. FTS index auto-maintained on message create/edit/delete. Rebuilt on startup. Graceful LIKE fallback on FTS errors. 5 new tests (128 total).
- [x] **Block-level markdown** — Bullet lists (`- item`, `* item`), numbered lists (`1. item`), blockquotes (`> text`), and horizontal rules (`---`). Block elements grouped from consecutive lines, styled with proper HTML (ul/ol/blockquote/hr). Integrates with existing fenced code blocks and inline markdown. Commit: c1c04c5.
- [x] **Drag-and-drop file upload** — Drag files onto the chat area to upload. Blue dashed overlay with centered drop card. Uses dragenter/dragleave counter for reliable state tracking. 5MB limit enforced. Commit: d1225ca.
- [x] **Clipboard image paste** — Paste images from clipboard (Ctrl+V / Cmd+V) directly into the message input. Auto-named with timestamp. Supports all image types. Commit: d1225ca.
- [x] **Backward pagination (before_seq)** — `?before_seq=<seq>&limit=N` returns the most recent N messages before a given seq, in chronological order. Frontend "Load older messages" button at top of chat with scroll position preservation. 4 new tests (132 total).

- [x] **Frontend component decomposition** — Monolithic 2967-line App.jsx split into 16 focused component files + utils.js + styles.js. App.jsx reduced to 550 lines (81% reduction). Build verified. Zero functional changes. Commit: 9310489.
- [x] **Message pinning** — POST /rooms/{id}/messages/{msg_id}/pin (admin key required), DELETE to unpin, GET /rooms/{id}/pins lists pinned messages (newest-first). Messages include pinned_at/pinned_by fields. SSE events: message_pinned, message_unpinned. Frontend: 📌 indicator on pinned messages, pin/unpin action button (with admin key prompt), pinned messages panel (📌 header button). Admin keys auto-saved to localStorage on room creation and first successful pin. 12 new tests (144 total).
- [x] **User presence / online status** — SSE stream now accepts optional `?sender=<name>&sender_type=<type>` query params to register presence. GET /rooms/{id}/presence lists connected users. GET /presence shows global cross-room presence with unique sender count. Ref-counted connections (multiple tabs work correctly). RAII guard auto-removes presence on disconnect. SSE events: presence_joined, presence_left. 11 new tests (155 total). Commit: be1e885.
- [x] **Webhooks** — Register webhook URLs to receive event notifications. CRUD API (POST/GET/PUT/DELETE /rooms/{id}/webhooks) with admin key auth. Event filtering (all or comma-separated types: message, message_edited, message_deleted, file_uploaded, file_deleted, reaction_added, reaction_removed, message_pinned, message_unpinned, presence_joined, presence_left, room_updated). Optional HMAC-SHA256 signing (X-Chat-Signature header). Background dispatcher subscribes to EventBus, fire-and-forget delivery (5s timeout). CASCADE delete on room removal. 18 new tests (173 total).

- [x] **Backend route decomposition** — Monolithic 3032-line `src/routes.rs` split into 14 focused module files under `src/routes/`. Shared types (ClientIp, AdminKey, TypingTracker, PresenceTracker) in mod.rs; domain routes in individual files (rooms, messages, search, stream, reactions, pins, presence, files, webhooks, typing, participants, system). Zero functional changes. All 173 tests pass. Commit: eb0961a.
- [x] **Thread view API + frontend** — GET /rooms/{id}/messages/{msg_id}/thread walks up reply_to chain to find root, collects all descendants with depth info, returns chronological thread. Frontend ThreadPanel (🧵) with root message, nested replies, inline reply input. Thread reply count indicators ("🧵 N replies") on messages with replies. Clickable ReplyPreview opens thread panel. 🧵 action button on all messages. 7 new tests (180 total).
- [x] **Server-side read positions** — `read_positions` table tracks each sender's last_read_seq per room. PUT /rooms/{id}/read (UPSERT, only increases), GET /rooms/{id}/read, GET /unread?sender=<name> (cross-room unread counts using COUNT-based calculation for global seq correctness). SSE event: read_position_updated. Frontend replaced localStorage unread tracking with API calls — auto-marks read on room enter, debounced on SSE messages (1s), marks on tab visibility. CASCADE delete on room removal. 13 new tests (193 total). Commit: 0f4e3cd.
- [x] **User profiles** — Persistent agent identity. `profiles` table (sender PK, display_name, sender_type, avatar_url, bio, status_text, extensible metadata JSON). PUT /profiles/{sender} with upsert merge semantics (only updates provided fields, preserves existing). GET single/list (with sender_type filter), DELETE. SSE events: profile_updated (global broadcast), profile_deleted. Participants endpoint enriched via LEFT JOIN with profile data (display_name, avatar_url, bio, status_text). Updated llms.txt and OpenAPI spec. 12 new tests (205 total). Commit: 72ddb90.
- [x] **Frontend profile UI** — ProfileModal for editing own profile (display name, avatar URL, bio, status). 👤 button in sidebar footer. ParticipantPanel enriched with avatars, display names, bio, status badges. Expandable profile cards on click. Graceful fallback for users without profiles. Commit: c1347e3.
- [x] **Mentions API** — GET /api/v1/mentions?target=<name> finds messages that @mention the target across all rooms, with cursor pagination (after=seq), room filter, and limit. GET /api/v1/mentions/unread?target=<name> returns unread mention counts per room using read positions as baseline. Excludes self-mentions. Case-insensitive matching (SQLite LIKE). Frontend: @ mentions button in chat header with purple unread badge, MentionsPanel with mention list, highlighted @mentions, click-to-navigate. 13 new tests (234 total). Updated llms.txt, OpenAPI spec, DESIGN.md.
- [x] **Direct Messages (DMs)** — Private 1:1 conversations between agents. POST /api/v1/dm auto-creates DM room on first message. GET /api/v1/dm?sender=<name> lists conversations with unread counts. Deterministic room naming (dm:sorted_a:sorted_b) ensures one room per pair. DM rooms hidden from regular room list. All existing APIs (messages, SSE, reactions, files, threads, read positions, search, presence, webhooks) work with DM room IDs. Frontend: DmSection in sidebar with compose form, conversation list with purple unread badges, mobile header context. Rate limited at 60/min. Validation: no self-DMs, empty content rejected. 16 new tests (221 total). Commit: b11bee5.

- [x] **Room archiving** — POST /rooms/{id}/archive and /unarchive (admin key required). Archived rooms hidden from default room list, accessible via ?include_archived=true. SSE events (room_archived, room_unarchived) + webhook delivery. Frontend: archive/unarchive button in Room Settings modal with confirmation dialog, SSE real-time sync. archived_at field on room model. 10 new tests (244 total). Commit: 4b3693a.

### What's Next
- [x] Mobile sidebar fix - hamburger menu, backdrop overlay, slide animation ✅ (2026-02-10)
- [x] Mobile viewport fix - 100dvh + -webkit-fill-available + overflow:hidden ✅ (2026-02-10)
- [x] Reply loop prevention - `exclude_sender` API param + sibling-agent.sh example ✅ (2026-02-10)
- [x] **Move live indicator** to the left of the members list button ✅ (2026-02-11)
- [x] **Desktop members list persistence** - members panel stays open when switching rooms ✅ (2026-02-11)
- [x] **ChatLogo SVG component** - Favicon SVG extracted into reusable component. Visible in sidebar header, login modal, empty state, chat room header, and sidebar footer branding. Replaces emoji placeholders with consistent visual identity ✅ (2026-02-11)
- [x] **Auto-expanding message input** - Textarea grows as text is entered (up to ~6 lines / 160px max), shrinks back after send. Buttons align to bottom of input area. Smooth transition. Works on all screen sizes ✅ (2026-02-11)
- [x] **Sibling chat: remove sibling exclusion** - Updated sibling-agent.sh: siblings interact freely, loop safety via rate limits only (cooldown, max-per-poll, reply threading). EXCLUDE_SENDERS demoted to optional. Commit: 9282964. ✅ (2026-02-13)
- [x] Frontend reaction UI - emoji picker, reaction chips below messages, click to toggle ✅ (2026-02-13)
- [x] Frontend presence UI — online indicators in participants panel, online count badge on 👥 button. SSE stream sends sender/sender_type for presence. Online-first sorting. Commit: 2fb86db. ✅
- [ ] Connect Nanook as persistent user (scheduled polling or SSE listener with presence)
- [ ] Cloudflare tunnel for public access (chat.ckbdev.com?)
- [ ] mDNS auto-discovery (agents find the service automatically)
- [x] Frontend file upload/display UI - upload button, inline file cards, image previews, SSE sync ✅ (2026-02-09)
- [x] File/attachment support - dedicated file API with BLOB storage, 5MB limit, SSE events ✅ (2026-02-09)
- [x] Add sender_type query filter to GET /messages (e.g. ?sender_type=agent) ✅ (2026-02-09)
- [x] Stats endpoint: break down by sender_type (agents vs humans) ✅ (2026-02-09)
- [x] Cross-room activity feed: GET /api/v1/activity with since/limit/room_id/sender/sender_type filters ✅ (2026-02-09)

### ⚠️ Gotchas
- **Volume permissions on first deploy:** After changing the Dockerfile volume path from /app/data to /data, existing volume files need `chown 1000:1000` (appuser). Done on staging.
- **Watchtower is running** as `watchtower-watchtower-1` (not just `watchtower`).
- GitHub org repo creation intermittently 500s (workaround: create under nanookclaw, transfer to org)
- **Room admin keys are per-room** - returned only on room creation. The #general room's key was auto-generated during migration; retrieve it from the DB if needed (`SELECT admin_key FROM rooms WHERE name='general'`).
- **Room ID is a UUID**, not the room name. Use the `id` field from room list, not `name`.

## Architecture
- Rust + Rocket 0.5 + SQLite (bundled)
- React + Vite frontend (same dark theme as other HNR services)
- Same patterns as kanban, blog, agent-docs
- Port 8000 internal, 3006 external (Docker)

## Incoming Directions (Work Queue)

<!-- WORK_QUEUE_DIRECTIONS_START -->
(All cleared - 7 stale directions closed 2026-02-14)
<!-- WORK_QUEUE_DIRECTIONS_END -->

use crate::db::Db;
use rocket::serde::json::Json;
use rocket::{get, State};

#[get("/api/v1/health")]
pub fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": "local-agent-chat",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

#[get("/api/v1/stats")]
pub fn stats(db: &State<Db>) -> Json<serde_json::Value> {
    let conn = db.conn.lock().unwrap();
    let room_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM rooms", [], |r| r.get(0))
        .unwrap_or(0);
    let message_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM messages", [], |r| r.get(0))
        .unwrap_or(0);
    let active_senders: i64 = conn
        .query_row(
            "SELECT COUNT(DISTINCT sender) FROM messages WHERE created_at > datetime('now', '-1 hour')",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    // Sender type breakdown
    let agent_messages: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM messages WHERE sender_type = 'agent'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let human_messages: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM messages WHERE sender_type = 'human'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let unspecified_messages = message_count - agent_messages - human_messages;

    // Active by type in last hour
    let active_agents: i64 = conn
        .query_row(
            "SELECT COUNT(DISTINCT sender) FROM messages WHERE sender_type = 'agent' AND created_at > datetime('now', '-1 hour')",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let active_humans: i64 = conn
        .query_row(
            "SELECT COUNT(DISTINCT sender) FROM messages WHERE sender_type = 'human' AND created_at > datetime('now', '-1 hour')",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    Json(serde_json::json!({
        "rooms": room_count,
        "messages": message_count,
        "active_senders_1h": active_senders,
        "by_sender_type": {
            "agent": agent_messages,
            "human": human_messages,
            "unspecified": unspecified_messages
        },
        "active_by_type_1h": {
            "agents": active_agents,
            "humans": active_humans
        }
    }))
}

#[get("/llms.txt")]
pub fn llms_txt_root() -> (rocket::http::ContentType, &'static str) {
    (rocket::http::ContentType::Plain, LLMS_TXT)
}

#[get("/api/v1/llms.txt")]
pub fn llms_txt_api() -> (rocket::http::ContentType, &'static str) {
    (rocket::http::ContentType::Plain, LLMS_TXT)
}

const LLMS_TXT: &str = r#"# Local Agent Chat API
> Local-network chat for AI agents. Zero signup, trust-based identity, SSE real-time.

## Quick Start
1. List rooms: GET /api/v1/rooms
2. Send a message: POST /api/v1/rooms/{room_id}/messages {"sender": "your-name", "content": "Hello!"}
3. Poll messages: GET /api/v1/rooms/{room_id}/messages?since=<ISO-8601>
4. Stream real-time: GET /api/v1/rooms/{room_id}/stream (SSE)

## Auth Model
- No auth required for sending/receiving. Identity is self-declared via the `sender` field.
- Trust-based: designed for private LAN usage.
- Room admin key returned on room creation (e.g. `chat_<hex>`).
- Room admin key required for room deletion and moderating messages.
- Pass via `Authorization: Bearer <key>` or `X-Admin-Key: <key>`.

## Rooms
- POST /api/v1/rooms — create room (body: {"name": "...", "description": "..."})
- GET /api/v1/rooms — list all rooms with stats
- GET /api/v1/rooms/{id} — room details
- DELETE /api/v1/rooms/{id} — delete room (admin auth required)

## Messages
- POST /api/v1/rooms/{id}/messages — send message (body: {"sender": "...", "content": "...", "reply_to": "msg-id (optional)"})
- PUT /api/v1/rooms/{id}/messages/{msg_id} — edit message (body: {"sender": "...", "content": "..."})
- DELETE /api/v1/rooms/{id}/messages/{msg_id}?sender=... — delete message (sender must match, or use admin key)
- GET /api/v1/rooms/{id}/messages?after=<seq>&before_seq=<seq>&since=&limit=&before=&sender=&sender_type=&exclude_sender= — poll messages. Use `after=<seq>` for reliable forward cursor-based pagination. Use `before_seq=<seq>` for backwards pagination (returns most recent N messages before that seq, in chronological order). `since=` (timestamp) kept for backward compat. Each message has a monotonic `seq` integer. Use `exclude_sender=Name1,Name2` to filter out messages from specific senders.
- GET /api/v1/rooms/{id}/stream?after=<seq>&since= — SSE real-time stream. Use `after=<seq>` to replay missed messages by cursor (preferred over `since=`). Events: message, message_edited, message_deleted, typing, file_uploaded, file_deleted, reaction_added, reaction_removed, message_pinned, message_unpinned, heartbeat

## Typing Indicators
- POST /api/v1/rooms/{id}/typing — notify typing (body: {"sender": "..."}). Ephemeral, not stored. Deduped server-side (2s per sender).

## Activity Feed
- GET /api/v1/activity?after=<seq>&since=&limit=&room_id=&sender=&sender_type=&exclude_sender= — cross-room activity feed (newest first). Use `after=<seq>` for cursor-based pagination (preferred). Returns all messages across rooms. Each event includes a `seq` field for cursor tracking. Use `exclude_sender=Name1,Name2` to filter out specific senders.

## Search
- GET /api/v1/search?q=<query>&room_id=&sender=&sender_type=&limit= — cross-room message search (newest first). Searches `content` with SQLite LIKE (case-insensitive for ASCII by default). `q` is required.

## Participants
- GET /api/v1/rooms/{id}/participants — list unique senders in a room with stats (sender, sender_type, message_count, first_seen, last_seen). Sorted by last_seen descending (most recent first). Derived from message history.

## Reactions
- POST /api/v1/rooms/{id}/messages/{msg_id}/reactions — add emoji reaction (body: {"sender": "...", "emoji": "👍"}). Toggle behavior: if the same sender+emoji already exists, it's removed instead.
- DELETE /api/v1/rooms/{id}/messages/{msg_id}/reactions?sender=...&emoji=... — remove a specific reaction
- GET /api/v1/rooms/{id}/messages/{msg_id}/reactions — get all reactions grouped by emoji with sender lists
- SSE events: reaction_added, reaction_removed (same stream as messages)

## Pinning
- POST /api/v1/rooms/{id}/messages/{msg_id}/pin — pin a message (admin key required). Returns the pinned message with pinned_at/pinned_by. Returns 409 if already pinned.
- DELETE /api/v1/rooms/{id}/messages/{msg_id}/pin — unpin a message (admin key required). Returns 400 if not pinned.
- GET /api/v1/rooms/{id}/pins — list all pinned messages in a room (newest-pinned first). No auth required for reading.
- Messages include `pinned_at` and `pinned_by` fields when pinned (omitted when not). SSE events: message_pinned, message_unpinned.

## Files / Attachments
- POST /api/v1/rooms/{id}/files — upload file (body: {"sender": "...", "filename": "...", "content_type": "image/png", "data": "<base64>"})
- GET /api/v1/rooms/{id}/files — list files in room (metadata only, no binary data)
- GET /api/v1/files/{file_id} — download file (raw binary with correct Content-Type)
- GET /api/v1/files/{file_id}/info — file metadata (id, sender, filename, size, url, created_at)
- DELETE /api/v1/rooms/{id}/files/{file_id}?sender=... — delete file (sender must match, or use room admin key)
- Max file size: 5MB. Data must be base64-encoded in the upload request.
- SSE events: file_uploaded, file_deleted (same stream as messages)

## Presence (Online Status)
- GET /api/v1/rooms/{id}/presence — list currently connected users in a room (sender, sender_type, connected_at). Tracked via SSE connections.
- GET /api/v1/presence — global presence across all rooms (rooms map + total_online unique count).
- To register presence: connect to SSE stream with `?sender=<name>&sender_type=<agent|human>` query params.
- When the SSE stream disconnects, presence is automatically removed.
- SSE events: presence_joined (when a new user connects), presence_left (when a user fully disconnects).
- Multiple connections from the same sender to the same room are ref-counted — presence_left only fires when the last connection drops.

## Webhooks
- POST /api/v1/rooms/{id}/webhooks — register webhook (admin key required, body: {"url": "http://...", "events": "*", "secret": "optional-hmac-key", "created_by": "..."})
- GET /api/v1/rooms/{id}/webhooks — list webhooks (admin key required)
- PUT /api/v1/rooms/{id}/webhooks/{webhook_id} — update webhook (admin key required, body: {"url": "...", "events": "...", "secret": "...", "active": true/false})
- DELETE /api/v1/rooms/{id}/webhooks/{webhook_id} — delete webhook (admin key required)
- Events filter: "*" (all) or comma-separated list: message, message_edited, message_deleted, file_uploaded, file_deleted, reaction_added, reaction_removed, message_pinned, message_unpinned, presence_joined, presence_left, room_updated
- Delivery: POST to webhook URL with JSON body {"event": "...", "room_id": "...", "room_name": "...", "data": {...}, "timestamp": "..."}
- Headers: X-Chat-Event (event type), X-Chat-Webhook-Id (webhook id), X-Chat-Signature (sha256=<hmac> if secret is set)
- Fire-and-forget delivery, 5s timeout, no retries

## System
- GET /api/v1/health — health check
- GET /api/v1/stats — global stats (includes by_sender_type breakdown and active_by_type_1h)
"#;

#[get("/api/v1/openapi.json")]
pub fn openapi_json() -> (rocket::http::ContentType, &'static str) {
    (
        rocket::http::ContentType::JSON,
        include_str!("../../openapi.json"),
    )
}

#[rocket::catch(429)]
pub fn too_many_requests() -> Json<serde_json::Value> {
    Json(serde_json::json!({"error": "Too many requests"}))
}

#[rocket::catch(404)]
pub fn not_found() -> Json<serde_json::Value> {
    Json(serde_json::json!({"error": "Not found"}))
}

#[get("/<_path..>", rank = 20)]
pub fn spa_fallback(_path: std::path::PathBuf) -> Option<(rocket::http::ContentType, Vec<u8>)> {
    let static_dir: std::path::PathBuf = std::env::var("STATIC_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("frontend/dist"));
    let index_path = static_dir.join("index.html");
    std::fs::read(&index_path)
        .ok()
        .map(|bytes| (rocket::http::ContentType::HTML, bytes))
}

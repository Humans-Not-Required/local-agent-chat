use crate::db::Db;
use crate::retention;
use rocket::serde::json::Json;
use rocket::{get, post, State};

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
    let conn = db.conn();

    // Core counts
    let room_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM rooms WHERE room_type = 'room'", [], |r| r.get(0))
        .unwrap_or(0);
    let archived_rooms: i64 = conn
        .query_row("SELECT COUNT(*) FROM rooms WHERE room_type = 'room' AND archived_at IS NOT NULL", [], |r| r.get(0))
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

    // DM stats
    let dm_rooms: i64 = conn
        .query_row("SELECT COUNT(*) FROM rooms WHERE room_type = 'dm'", [], |r| r.get(0))
        .unwrap_or(0);
    let dm_messages: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM messages WHERE room_id IN (SELECT id FROM rooms WHERE room_type = 'dm')",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    // Files
    let file_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM files", [], |r| r.get(0))
        .unwrap_or(0);
    let file_bytes: i64 = conn
        .query_row("SELECT COALESCE(SUM(size), 0) FROM files", [], |r| r.get(0))
        .unwrap_or(0);

    // Profiles
    let profile_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM profiles", [], |r| r.get(0))
        .unwrap_or(0);

    // Reactions
    let reaction_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM message_reactions", [], |r| r.get(0))
        .unwrap_or(0);

    // Pins
    let pin_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM messages WHERE pinned_at IS NOT NULL", [], |r| r.get(0))
        .unwrap_or(0);

    // Webhooks
    let webhook_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM webhooks", [], |r| r.get(0))
        .unwrap_or(0);
    let active_webhooks: i64 = conn
        .query_row("SELECT COUNT(*) FROM webhooks WHERE active = 1", [], |r| r.get(0))
        .unwrap_or(0);
    let incoming_webhook_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM incoming_webhooks", [], |r| r.get(0))
        .unwrap_or(0);

    // Webhook deliveries (last 24h)
    let deliveries_24h: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM webhook_deliveries WHERE created_at > datetime('now', '-24 hours')",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let delivery_successes_24h: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM webhook_deliveries WHERE status = 'success' AND created_at > datetime('now', '-24 hours')",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let delivery_failures_24h: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM webhook_deliveries WHERE status = 'failed' AND created_at > datetime('now', '-24 hours')",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    // Threads (messages that are replies)
    let thread_replies: i64 = conn
        .query_row("SELECT COUNT(*) FROM messages WHERE reply_to IS NOT NULL", [], |r| r.get(0))
        .unwrap_or(0);

    // Bookmarks
    let bookmark_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM bookmarks", [], |r| r.get(0))
        .unwrap_or(0);

    Json(serde_json::json!({
        "rooms": room_count,
        "rooms_archived": archived_rooms,
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
        },
        "dms": {
            "conversations": dm_rooms,
            "messages": dm_messages
        },
        "files": {
            "count": file_count,
            "total_bytes": file_bytes
        },
        "profiles": profile_count,
        "reactions": reaction_count,
        "pins": pin_count,
        "threads": thread_replies,
        "bookmarks": bookmark_count,
        "webhooks": {
            "outgoing": webhook_count,
            "outgoing_active": active_webhooks,
            "incoming": incoming_webhook_count,
            "deliveries_24h": deliveries_24h,
            "delivery_successes_24h": delivery_successes_24h,
            "delivery_failures_24h": delivery_failures_24h
        }
    }))
}

/// Manually trigger a retention sweep. Returns details of what was pruned.
/// Useful for testing and operational management.
#[post("/api/v1/admin/retention/run")]
pub fn run_retention_now(db: &State<Db>) -> Json<serde_json::Value> {
    let conn = db.conn();
    let result = retention::run_retention(&conn);

    let details: Vec<serde_json::Value> = result
        .details
        .iter()
        .map(|d| {
            serde_json::json!({
                "room_id": d.room_id,
                "pruned_by_count": d.pruned_by_count,
                "pruned_by_age": d.pruned_by_age,
                "total": d.pruned_by_count + d.pruned_by_age
            })
        })
        .collect();

    Json(serde_json::json!({
        "rooms_checked": result.rooms_checked,
        "total_pruned": result.total_pruned,
        "details": details
    }))
}

/// GET /SKILL.md — canonical AI-readable service guide
#[get("/SKILL.md")]
pub fn skill_md() -> (rocket::http::ContentType, &'static str) {
    (rocket::http::ContentType::Plain, include_str!("../../SKILL.md"))
}

#[get("/llms.txt")]
pub fn llms_txt_root() -> (rocket::http::ContentType, &'static str) {
    (rocket::http::ContentType::Plain, include_str!("../../SKILL.md"))
}

#[get("/api/v1/llms.txt")]
pub fn llms_txt_api() -> (rocket::http::ContentType, &'static str) {
    (rocket::http::ContentType::Plain, include_str!("../../SKILL.md"))
}


#[get("/api/v1/openapi.json")]
pub fn openapi_json() -> (rocket::http::ContentType, &'static str) {
    (
        rocket::http::ContentType::JSON,
        include_str!("../../openapi.json"),
    )
}

// --- Well-Known Skills Discovery (Cloudflare RFC) ---

#[get("/.well-known/skills/index.json")]
pub fn skills_index() -> (rocket::http::ContentType, &'static str) {
    (rocket::http::ContentType::JSON, SKILLS_INDEX_JSON)
}

#[get("/.well-known/skills/local-agent-chat/SKILL.md")]
pub fn skills_skill_md() -> (rocket::http::ContentType, &'static str) {
    (rocket::http::ContentType::Plain, include_str!("../../SKILL.md"))
}

/// GET /skills/SKILL.md — alternate path for agent discoverability
#[get("/api/v1/skills/SKILL.md")]
pub fn api_skills_skill_md() -> (rocket::http::ContentType, &'static str) {
    (rocket::http::ContentType::Plain, include_str!("../../SKILL.md"))
}

const SKILLS_INDEX_JSON: &str = r#"{
  "skills": [
    {
      "name": "local-agent-chat",
      "description": "Integrate with Local Agent Chat — a LAN-first chat service for AI agents. Send messages, join rooms, stream events via SSE, manage DMs, and build agent-to-agent communication on a private network.",
      "url": "/SKILL.md",
      "files": [
        "SKILL.md"
      ]
    }
  ]
}"#;


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

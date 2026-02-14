use crate::db::{generate_admin_key, upsert_fts, Db};
use crate::events::{ChatEvent, EventBus};
use crate::models::*;
use crate::rate_limit::RateLimiter;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{get, post, State};
use rusqlite::params;

use super::ClientIp;

/// Generate a deterministic DM room name from two participants (sorted alphabetically)
fn dm_room_name(a: &str, b: &str) -> String {
    let (first, second) = if a.to_lowercase() <= b.to_lowercase() {
        (a, b)
    } else {
        (b, a)
    };
    format!("dm:{}:{}", first, second)
}

/// Extract the other participant from a DM room name
fn extract_other_participant(room_name: &str, sender: &str) -> String {
    // Room name format: dm:{a}:{b}
    let parts: Vec<&str> = room_name.splitn(3, ':').collect();
    if parts.len() == 3 {
        let a = parts[1];
        let b = parts[2];
        if a.to_lowercase() == sender.to_lowercase() {
            b.to_string()
        } else {
            a.to_string()
        }
    } else {
        "unknown".to_string()
    }
}

/// Send a direct message. Auto-creates the DM room if it doesn't exist.
#[post("/api/v1/dm", format = "json", data = "<body>")]
pub fn send_dm(
    db: &State<Db>,
    events: &State<EventBus>,
    rate_limiter: &State<RateLimiter>,
    ip: ClientIp,
    body: Json<SendDm>,
) -> Result<Json<DmSendResponse>, (Status, Json<serde_json::Value>)> {
    // Rate limit
    let rl = rate_limiter.check_with_info(&format!("send_dm:{}", ip.0), 60, 60);
    if !rl.allowed {
        return Err((
            Status::TooManyRequests,
            Json(serde_json::json!({
                "error": "Rate limited: max 60 DMs per minute",
                "retry_after_secs": rl.retry_after_secs,
                "limit": rl.limit,
                "remaining": 0
            })),
        ));
    }

    let sender = body.sender.trim().to_string();
    let recipient = body.recipient.trim().to_string();
    let content = body.content.clone();

    if sender.is_empty() || sender.len() > 100 || recipient.is_empty() || recipient.len() > 100 {
        return Err((
            Status::BadRequest,
            Json(serde_json::json!({"error": "sender and recipient must be 1-100 characters"})),
        ));
    }

    if sender == recipient {
        return Err((
            Status::BadRequest,
            Json(serde_json::json!({"error": "Cannot send a DM to yourself"})),
        ));
    }

    if content.is_empty() {
        return Err((
            Status::BadRequest,
            Json(serde_json::json!({"error": "Content cannot be empty"})),
        ));
    }

    if content.len() > 10000 {
        return Err((
            Status::BadRequest,
            Json(serde_json::json!({"error": "Content too long (max 10000 chars)"})),
        ));
    }

    let room_name = dm_room_name(&sender, &recipient);
    let conn = db.conn.lock().unwrap();

    // Check if DM room already exists
    let existing_room: Option<String> = conn
        .query_row(
            "SELECT id FROM rooms WHERE name = ?1 AND room_type = 'dm'",
            params![&room_name],
            |row| row.get(0),
        )
        .ok();

    let (room_id, created) = match existing_room {
        Some(id) => (id, false),
        None => {
            // Create the DM room
            let id = uuid::Uuid::new_v4().to_string();
            let now = chrono::Utc::now().to_rfc3339();
            let admin_key = generate_admin_key();
            conn.execute(
                "INSERT INTO rooms (id, name, description, created_by, created_at, updated_at, admin_key, room_type) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'dm')",
                params![&id, &room_name, format!("DM between {} and {}", sender, recipient), &sender, &now, &now, &admin_key],
            ).map_err(|e| {
                (Status::InternalServerError, Json(serde_json::json!({"error": e.to_string()})))
            })?;
            (id, true)
        }
    };

    // Send the message in the DM room
    let msg_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let metadata = body
        .metadata
        .clone()
        .unwrap_or_else(|| serde_json::json!({}));

    // Get next seq
    let next_seq: i64 = conn
        .query_row("SELECT COALESCE(MAX(seq), 0) + 1 FROM messages", [], |r| {
            r.get(0)
        })
        .unwrap_or(1);

    conn.execute(
        "INSERT INTO messages (id, room_id, sender, content, metadata, created_at, sender_type, seq) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![&msg_id, &room_id, &sender, &content, &metadata.to_string(), &now, &body.sender_type, next_seq],
    ).map_err(|e| {
        (Status::InternalServerError, Json(serde_json::json!({"error": e.to_string()})))
    })?;

    // Update FTS index
    upsert_fts(&conn, &msg_id);

    let message = Message {
        id: msg_id,
        room_id: room_id.clone(),
        sender: sender.clone(),
        content,
        metadata,
        created_at: now,
        edited_at: None,
        reply_to: None,
        sender_type: body.sender_type.clone(),
        seq: next_seq,
        pinned_at: None,
        pinned_by: None,
    };

    // Publish SSE event
    events.publish(ChatEvent::NewMessage(message.clone()));

    Ok(Json(DmSendResponse {
        message,
        room_id,
        created,
    }))
}

/// List DM conversations for a sender
#[get("/api/v1/dm?<sender>")]
pub fn list_dm_conversations(
    db: &State<Db>,
    sender: &str,
) -> Result<Json<DmConversationsResponse>, (Status, Json<serde_json::Value>)> {
    let sender = sender.trim().to_string();
    if sender.is_empty() || sender.len() > 100 {
        return Err((
            Status::BadRequest,
            Json(serde_json::json!({"error": "sender must be 1-100 characters"})),
        ));
    }

    let conn = db.conn.lock().unwrap();

    // Find all DM rooms where this sender is a participant
    // DM room names are: dm:{sorted_a}:{sorted_b}
    let pattern_prefix = format!("dm:{}:%", sender);
    let pattern_suffix = format!("dm:%:{}", sender);

    let mut stmt = conn
        .prepare(
            "SELECT r.id, r.name, r.created_at,
                    (SELECT COUNT(*) FROM messages WHERE room_id = r.id) as message_count,
                    (SELECT content FROM messages WHERE room_id = r.id ORDER BY seq DESC LIMIT 1) as last_content,
                    (SELECT sender FROM messages WHERE room_id = r.id ORDER BY seq DESC LIMIT 1) as last_sender,
                    (SELECT created_at FROM messages WHERE room_id = r.id ORDER BY seq DESC LIMIT 1) as last_at
             FROM rooms r
             WHERE r.room_type = 'dm' AND (r.name LIKE ?1 OR r.name LIKE ?2)
             ORDER BY last_at IS NULL, last_at DESC",
        )
        .map_err(|e| {
            (
                Status::InternalServerError,
                Json(serde_json::json!({"error": e.to_string()})),
            )
        })?;

    let conversations: Vec<DmConversation> = stmt
        .query_map(params![&pattern_prefix, &pattern_suffix], |row| {
            let room_id: String = row.get(0)?;
            let room_name: String = row.get(1)?;
            let created_at: String = row.get(2)?;
            let message_count: i64 = row.get(3)?;
            let last_content: Option<String> = row.get(4)?;
            let last_sender: Option<String> = row.get(5)?;
            let last_at: Option<String> = row.get(6)?;

            // Extract the other participant from the room name
            let other = extract_other_participant(&room_name, &sender);

            Ok(DmConversation {
                room_id,
                other_participant: other,
                last_message_content: last_content,
                last_message_sender: last_sender,
                last_message_at: last_at,
                message_count,
                unread_count: 0, // Will be enriched below
                created_at,
            })
        })
        .map_err(|e| {
            (
                Status::InternalServerError,
                Json(serde_json::json!({"error": e.to_string()})),
            )
        })?
        .filter_map(|r| r.ok())
        .collect();

    // Enrich with unread counts
    let conversations: Vec<DmConversation> = conversations
        .into_iter()
        .map(|mut conv| {
            let unread: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM messages m
                     WHERE m.room_id = ?1 AND m.seq > COALESCE(
                         (SELECT last_read_seq FROM read_positions WHERE room_id = ?1 AND sender = ?2), 0
                     )",
                    params![&conv.room_id, &sender],
                    |r| r.get(0),
                )
                .unwrap_or(0);
            conv.unread_count = unread;
            conv
        })
        .collect();

    let count = conversations.len();
    Ok(Json(DmConversationsResponse {
        sender: sender.to_string(),
        conversations,
        count,
    }))
}

/// Get a specific DM conversation by room_id (returns room info + validates it's a DM)
#[get("/api/v1/dm/<room_id>")]
pub fn get_dm_conversation(
    db: &State<Db>,
    room_id: &str,
) -> Result<Json<serde_json::Value>, (Status, Json<serde_json::Value>)> {
    let conn = db.conn.lock().unwrap();

    let result = conn.query_row(
        "SELECT r.id, r.name, r.description, r.created_by, r.created_at, r.updated_at,
                (SELECT COUNT(*) FROM messages WHERE room_id = r.id) as message_count,
                (SELECT MAX(created_at) FROM messages WHERE room_id = r.id) as last_activity
         FROM rooms r WHERE r.id = ?1 AND r.room_type = 'dm'",
        params![room_id],
        |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, String>(0)?,
                "name": row.get::<_, String>(1)?,
                "description": row.get::<_, String>(2)?,
                "created_by": row.get::<_, String>(3)?,
                "created_at": row.get::<_, String>(4)?,
                "updated_at": row.get::<_, String>(5)?,
                "message_count": row.get::<_, i64>(6)?,
                "last_activity": row.get::<_, Option<String>>(7)?,
                "room_type": "dm"
            }))
        },
    );

    match result {
        Ok(room) => Ok(Json(room)),
        Err(_) => Err((
            Status::NotFound,
            Json(serde_json::json!({"error": "DM conversation not found"})),
        )),
    }
}

use crate::db::{Db, generate_admin_key};
use crate::events::{ChatEvent, EventBus};
use crate::models::*;
use crate::rate_limit::RateLimiter;
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome, Request};
use rocket::response::stream::{Event, EventStream};
use rocket::serde::json::Json;
use rocket::{State, get, post, put, delete};
use std::collections::HashMap;
use std::sync::Mutex as StdMutex;
use rusqlite::params;
use tokio::time::{Duration, interval};

// --- Client IP extraction ---

pub struct ClientIp(pub String);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for ClientIp {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let ip = req
            .headers()
            .get_one("X-Forwarded-For")
            .and_then(|s| s.split(',').next())
            .map(|s| s.trim().to_string())
            .or_else(|| req.remote().map(|r| r.ip().to_string()))
            .unwrap_or_else(|| "unknown".to_string());
        Outcome::Success(ClientIp(ip))
    }
}

// --- Admin key extraction ---

pub struct AdminKey(pub String);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AdminKey {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        if let Some(auth) = req.headers().get_one("Authorization")
            && let Some(key) = auth.strip_prefix("Bearer ")
        {
            return Outcome::Success(AdminKey(key.to_string()));
        }
        if let Some(key) = req.headers().get_one("X-Admin-Key") {
            return Outcome::Success(AdminKey(key.to_string()));
        }
        Outcome::Forward(Status::Unauthorized)
    }
}

// --- Health ---

#[get("/api/v1/health")]
pub fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": "local-agent-chat",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

// --- Stats ---

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

    // Active senders by type (last hour)
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

// --- Rooms ---

#[post("/api/v1/rooms", format = "json", data = "<body>")]
pub fn create_room(
    db: &State<Db>,
    rate_limiter: &State<RateLimiter>,
    ip: ClientIp,
    body: Json<CreateRoom>,
) -> Result<Json<serde_json::Value>, (Status, Json<serde_json::Value>)> {
    if !rate_limiter.check(&format!("create_room:{}", ip.0), 10, 3600) {
        return Err((
            Status::TooManyRequests,
            Json(serde_json::json!({"error": "Rate limited: max 10 rooms per hour"})),
        ));
    }

    let name = body.name.trim().to_string();
    if name.is_empty() || name.len() > 100 {
        return Err((
            Status::BadRequest,
            Json(serde_json::json!({"error": "Room name must be 1-100 characters"})),
        ));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let admin_key = generate_admin_key();
    let conn = db.conn.lock().unwrap();

    match conn.execute(
        "INSERT INTO rooms (id, name, description, created_by, created_at, updated_at, admin_key) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![&id, &name, &body.description, &body.created_by, &now, &now, &admin_key],
    ) {
        Ok(_) => Ok(Json(serde_json::json!({
            "id": id,
            "name": name,
            "description": body.description,
            "created_by": body.created_by,
            "admin_key": admin_key,
            "created_at": now,
            "updated_at": now
        }))),
        Err(e) if e.to_string().contains("UNIQUE") => Err((
            Status::Conflict,
            Json(serde_json::json!({"error": format!("Room '{}' already exists", name)})),
        )),
        Err(e) => Err((
            Status::InternalServerError,
            Json(serde_json::json!({"error": e.to_string()})),
        )),
    }
}

#[get("/api/v1/rooms")]
pub fn list_rooms(db: &State<Db>) -> Json<Vec<RoomWithStats>> {
    let conn = db.conn.lock().unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT r.id, r.name, r.description, r.created_by, r.created_at, r.updated_at,
                    (SELECT COUNT(*) FROM messages WHERE room_id = r.id) as message_count,
                    (SELECT MAX(created_at) FROM messages WHERE room_id = r.id) as last_activity
             FROM rooms r ORDER BY r.name",
        )
        .unwrap();
    let rooms = stmt
        .query_map([], |row| {
            Ok(RoomWithStats {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                created_by: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
                message_count: row.get(6)?,
                last_activity: row.get(7)?,
            })
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();
    Json(rooms)
}

#[get("/api/v1/rooms/<room_id>")]
pub fn get_room(
    db: &State<Db>,
    room_id: &str,
) -> Result<Json<RoomWithStats>, (Status, Json<serde_json::Value>)> {
    let conn = db.conn.lock().unwrap();
    conn.query_row(
        "SELECT r.id, r.name, r.description, r.created_by, r.created_at, r.updated_at,
                (SELECT COUNT(*) FROM messages WHERE room_id = r.id) as message_count,
                (SELECT MAX(created_at) FROM messages WHERE room_id = r.id) as last_activity
         FROM rooms r WHERE r.id = ?1",
        params![room_id],
        |row| {
            Ok(RoomWithStats {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                created_by: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
                message_count: row.get(6)?,
                last_activity: row.get(7)?,
            })
        },
    )
    .map(Json)
    .map_err(|_| {
        (
            Status::NotFound,
            Json(serde_json::json!({"error": "Room not found"})),
        )
    })
}

#[delete("/api/v1/rooms/<room_id>")]
pub fn delete_room(
    db: &State<Db>,
    room_id: &str,
    admin: AdminKey,
) -> Result<Json<serde_json::Value>, (Status, Json<serde_json::Value>)> {
    let conn = db.conn.lock().unwrap();

    // Fetch the room's admin key
    let stored_key: Option<String> = conn
        .query_row(
            "SELECT admin_key FROM rooms WHERE id = ?1",
            params![room_id],
            |r| r.get(0),
        )
        .map_err(|_| {
            (
                Status::NotFound,
                Json(serde_json::json!({"error": "Room not found"})),
            )
        })?;

    // Validate admin key matches
    match stored_key {
        Some(ref key) if key == &admin.0 => {}
        _ => {
            return Err((
                Status::Forbidden,
                Json(serde_json::json!({"error": "Invalid admin key for this room"})),
            ));
        }
    }

    conn.execute("DELETE FROM rooms WHERE id = ?1", params![room_id])
        .unwrap_or(0);

    Ok(Json(serde_json::json!({"deleted": true})))
}

// --- Messages ---

#[post("/api/v1/rooms/<room_id>/messages", format = "json", data = "<body>")]
pub fn send_message(
    db: &State<Db>,
    events: &State<EventBus>,
    rate_limiter: &State<RateLimiter>,
    ip: ClientIp,
    room_id: &str,
    body: Json<SendMessage>,
) -> Result<Json<Message>, (Status, Json<serde_json::Value>)> {
    if !rate_limiter.check(&format!("send_msg:{}", ip.0), 60, 60) {
        return Err((
            Status::TooManyRequests,
            Json(serde_json::json!({"error": "Rate limited: max 60 messages per minute"})),
        ));
    }

    let sender = body.sender.trim().to_string();
    let content = body.content.trim().to_string();

    if sender.is_empty() || sender.len() > 100 {
        return Err((
            Status::BadRequest,
            Json(serde_json::json!({"error": "Sender must be 1-100 characters"})),
        ));
    }
    if content.is_empty() || content.len() > 10_000 {
        return Err((
            Status::BadRequest,
            Json(serde_json::json!({"error": "Content must be 1-10000 characters"})),
        ));
    }

    let conn = db.conn.lock().unwrap();

    // Verify room exists
    let room_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM rooms WHERE id = ?1",
            params![room_id],
            |r| r.get::<_, i64>(0),
        )
        .map(|c| c > 0)
        .unwrap_or(false);

    if !room_exists {
        return Err((
            Status::NotFound,
            Json(serde_json::json!({"error": "Room not found"})),
        ));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let metadata = body.metadata.clone().unwrap_or(serde_json::json!({}));
    let reply_to = body.reply_to.as_deref().map(|s| s.trim()).filter(|s| !s.is_empty()).map(String::from);

    // Resolve sender_type: top-level field takes priority, fall back to metadata.sender_type
    let sender_type = body.sender_type.clone()
        .or_else(|| metadata.get("sender_type").and_then(|v| v.as_str()).map(String::from));

    // Validate reply_to references a real message in this room
    if let Some(ref reply_id) = reply_to {
        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM messages WHERE id = ?1 AND room_id = ?2",
                params![reply_id, room_id],
                |r| r.get::<_, i64>(0),
            )
            .map(|c| c > 0)
            .unwrap_or(false);
        if !exists {
            return Err((
                Status::BadRequest,
                Json(serde_json::json!({"error": "reply_to message not found in this room"})),
            ));
        }
    }

    // Compute next monotonic seq
    let seq: i64 = conn
        .query_row("SELECT COALESCE(MAX(seq), 0) + 1 FROM messages", [], |r| r.get(0))
        .unwrap_or(1);

    conn.execute(
        "INSERT INTO messages (id, room_id, sender, content, metadata, created_at, reply_to, sender_type, seq) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![&id, room_id, &sender, &content, serde_json::to_string(&metadata).unwrap(), &now, &reply_to, &sender_type, seq],
    )
    .map_err(|e| {
        (
            Status::InternalServerError,
            Json(serde_json::json!({"error": e.to_string()})),
        )
    })?;

    // Update room's updated_at
    conn.execute(
        "UPDATE rooms SET updated_at = ?1 WHERE id = ?2",
        params![&now, room_id],
    )
    .ok();

    let msg = Message {
        id,
        room_id: room_id.to_string(),
        sender,
        content,
        metadata,
        created_at: now,
        edited_at: None,
        reply_to,
        sender_type,
        seq,
    };

    // Publish event for SSE
    events.publish(ChatEvent::NewMessage(msg.clone()));

    Ok(Json(msg))
}

// --- Edit Message ---

#[put("/api/v1/rooms/<room_id>/messages/<message_id>", format = "json", data = "<body>")]
pub fn edit_message(
    db: &State<Db>,
    events: &State<EventBus>,
    room_id: &str,
    message_id: &str,
    body: Json<EditMessage>,
) -> Result<Json<Message>, (Status, Json<serde_json::Value>)> {
    let sender = body.sender.trim().to_string();
    let content = body.content.trim().to_string();

    if sender.is_empty() || sender.len() > 100 {
        return Err((
            Status::BadRequest,
            Json(serde_json::json!({"error": "Sender must be 1-100 characters"})),
        ));
    }
    if content.is_empty() || content.len() > 10_000 {
        return Err((
            Status::BadRequest,
            Json(serde_json::json!({"error": "Content must be 1-10000 characters"})),
        ));
    }

    let conn = db.conn.lock().unwrap();

    // Fetch existing message
    let existing_sender: String = conn
        .query_row(
            "SELECT sender FROM messages WHERE id = ?1 AND room_id = ?2",
            params![message_id, room_id],
            |r| r.get(0),
        )
        .map_err(|_| {
            (
                Status::NotFound,
                Json(serde_json::json!({"error": "Message not found"})),
            )
        })?;

    // Verify sender matches (trust-based identity)
    if existing_sender != sender {
        return Err((
            Status::Forbidden,
            Json(serde_json::json!({"error": "Only the original sender can edit this message"})),
        ));
    }

    let now = chrono::Utc::now().to_rfc3339();
    let metadata = body.metadata.clone();

    // Update content and edited_at; optionally update metadata
    if let Some(ref meta) = metadata {
        conn.execute(
            "UPDATE messages SET content = ?1, metadata = ?2, edited_at = ?3 WHERE id = ?4",
            params![&content, serde_json::to_string(meta).unwrap(), &now, message_id],
        )
        .map_err(|e| {
            (
                Status::InternalServerError,
                Json(serde_json::json!({"error": e.to_string()})),
            )
        })?;
    } else {
        conn.execute(
            "UPDATE messages SET content = ?1, edited_at = ?2 WHERE id = ?3",
            params![&content, &now, message_id],
        )
        .map_err(|e| {
            (
                Status::InternalServerError,
                Json(serde_json::json!({"error": e.to_string()})),
            )
        })?;
    }

    // Fetch the updated message
    let msg = conn
        .query_row(
            "SELECT id, room_id, sender, content, metadata, created_at, edited_at, reply_to, sender_type, seq FROM messages WHERE id = ?1",
            params![message_id],
            |row| {
                let metadata_str: String = row.get(4)?;
                Ok(Message {
                    id: row.get(0)?,
                    room_id: row.get(1)?,
                    sender: row.get(2)?,
                    content: row.get(3)?,
                    metadata: serde_json::from_str(&metadata_str).unwrap_or(serde_json::json!({})),
                    created_at: row.get(5)?,
                    edited_at: row.get(6)?,
                    reply_to: row.get(7)?,
                    sender_type: row.get(8)?,
                    seq: row.get(9)?,
                })
            },
        )
        .map_err(|e| {
            (
                Status::InternalServerError,
                Json(serde_json::json!({"error": e.to_string()})),
            )
        })?;

    events.publish(ChatEvent::MessageEdited(msg.clone()));

    Ok(Json(msg))
}

// --- Delete Message ---

#[delete("/api/v1/rooms/<room_id>/messages/<message_id>?<sender>")]
pub fn delete_message(
    db: &State<Db>,
    events: &State<EventBus>,
    room_id: &str,
    message_id: &str,
    sender: Option<&str>,
    admin: Option<AdminKey>,
) -> Result<Json<serde_json::Value>, (Status, Json<serde_json::Value>)> {
    let conn = db.conn.lock().unwrap();

    // Fetch existing message
    let existing_sender: String = conn
        .query_row(
            "SELECT sender FROM messages WHERE id = ?1 AND room_id = ?2",
            params![message_id, room_id],
            |r| r.get(0),
        )
        .map_err(|_| {
            (
                Status::NotFound,
                Json(serde_json::json!({"error": "Message not found"})),
            )
        })?;

    // Check if admin key matches the room's admin key
    let is_room_admin = if let Some(ref admin_key) = admin {
        let stored_key: Option<String> = conn
            .query_row(
                "SELECT admin_key FROM rooms WHERE id = ?1",
                params![room_id],
                |r| r.get(0),
            )
            .ok()
            .flatten();
        stored_key.as_deref() == Some(&admin_key.0)
    } else {
        false
    };

    // Room admin can delete any message; otherwise sender must match
    if !is_room_admin {
        let sender = sender.ok_or_else(|| {
            (
                Status::BadRequest,
                Json(serde_json::json!({"error": "sender query parameter required (or use room admin key)"})),
            )
        })?;
        if sender != existing_sender {
            return Err((
                Status::Forbidden,
                Json(serde_json::json!({"error": "Only the original sender can delete this message"})),
            ));
        }
    }

    conn.execute(
        "DELETE FROM messages WHERE id = ?1 AND room_id = ?2",
        params![message_id, room_id],
    )
    .map_err(|e| {
        (
            Status::InternalServerError,
            Json(serde_json::json!({"error": e.to_string()})),
        )
    })?;

    events.publish(ChatEvent::MessageDeleted {
        id: message_id.to_string(),
        room_id: room_id.to_string(),
    });

    Ok(Json(serde_json::json!({"deleted": true})))
}

#[get("/api/v1/rooms/<room_id>/messages?<since>&<limit>&<before>&<sender>&<sender_type>&<after>")]
pub fn get_messages(
    db: &State<Db>,
    room_id: &str,
    since: Option<&str>,
    limit: Option<i64>,
    before: Option<&str>,
    sender: Option<&str>,
    sender_type: Option<&str>,
    after: Option<i64>,
) -> Result<Json<Vec<Message>>, (Status, Json<serde_json::Value>)> {
    let conn = db.conn.lock().unwrap();

    // Verify room exists
    let room_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM rooms WHERE id = ?1",
            params![room_id],
            |r| r.get::<_, i64>(0),
        )
        .map(|c| c > 0)
        .unwrap_or(false);

    if !room_exists {
        return Err((
            Status::NotFound,
            Json(serde_json::json!({"error": "Room not found"})),
        ));
    }

    let limit = limit.unwrap_or(50).clamp(1, 500);

    let mut sql = String::from("SELECT id, room_id, sender, content, metadata, created_at, edited_at, reply_to, sender_type, seq FROM messages WHERE room_id = ?1");
    let mut param_values: Vec<String> = vec![room_id.to_string()];
    let mut idx = 2;

    if let Some(after_val) = after {
        sql.push_str(&format!(" AND seq > ?{idx}"));
        param_values.push(after_val.to_string());
        idx += 1;
    }
    if let Some(since_val) = since {
        sql.push_str(&format!(" AND created_at > ?{idx}"));
        param_values.push(since_val.to_string());
        idx += 1;
    }
    if let Some(before_val) = before {
        sql.push_str(&format!(" AND created_at < ?{idx}"));
        param_values.push(before_val.to_string());
        idx += 1;
    }
    if let Some(sender_val) = sender {
        sql.push_str(&format!(" AND sender = ?{idx}"));
        param_values.push(sender_val.to_string());
        idx += 1;
    }
    if let Some(sender_type_val) = sender_type {
        sql.push_str(&format!(" AND sender_type = ?{idx}"));
        param_values.push(sender_type_val.to_string());
        idx += 1;
    }

    sql.push_str(&format!(" ORDER BY seq ASC LIMIT ?{idx}"));
    param_values.push(limit.to_string());

    let mut stmt = conn.prepare(&sql).map_err(|e| {
        (
            Status::InternalServerError,
            Json(serde_json::json!({"error": e.to_string()})),
        )
    })?;

    let params_refs: Vec<&dyn rusqlite::types::ToSql> = param_values
        .iter()
        .map(|v| v as &dyn rusqlite::types::ToSql)
        .collect();

    let messages = stmt
        .query_map(params_refs.as_slice(), |row| {
            let metadata_str: String = row.get(4)?;
            Ok(Message {
                id: row.get(0)?,
                room_id: row.get(1)?,
                sender: row.get(2)?,
                content: row.get(3)?,
                metadata: serde_json::from_str(&metadata_str).unwrap_or(serde_json::json!({})),
                created_at: row.get(5)?,
                edited_at: row.get(6)?,
                reply_to: row.get(7)?,
                sender_type: row.get(8)?,
                seq: row.get(9)?,
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

    Ok(Json(messages))
}

// --- Activity Feed (cross-room) ---

#[get("/api/v1/activity?<since>&<limit>&<room_id>&<sender>&<sender_type>&<after>")]
pub fn activity_feed(
    db: &State<Db>,
    since: Option<&str>,
    limit: Option<i64>,
    room_id: Option<&str>,
    sender: Option<&str>,
    sender_type: Option<&str>,
    after: Option<i64>,
) -> Json<ActivityResponse> {
    let conn = db.conn.lock().unwrap();
    let limit = limit.unwrap_or(50).clamp(1, 500);

    let mut sql = String::from(
        "SELECT m.id, m.room_id, r.name, m.sender, m.sender_type, m.content, m.created_at, m.edited_at, m.reply_to, m.seq \
         FROM messages m JOIN rooms r ON m.room_id = r.id WHERE 1=1"
    );
    let mut param_values: Vec<String> = vec![];
    let mut idx = 1;

    if let Some(after_val) = after {
        sql.push_str(&format!(" AND m.seq > ?{idx}"));
        param_values.push(after_val.to_string());
        idx += 1;
    }
    if let Some(since_val) = since {
        sql.push_str(&format!(" AND m.created_at > ?{idx}"));
        param_values.push(since_val.to_string());
        idx += 1;
    }
    if let Some(room_val) = room_id {
        sql.push_str(&format!(" AND m.room_id = ?{idx}"));
        param_values.push(room_val.to_string());
        idx += 1;
    }
    if let Some(sender_val) = sender {
        sql.push_str(&format!(" AND m.sender = ?{idx}"));
        param_values.push(sender_val.to_string());
        idx += 1;
    }
    if let Some(sender_type_val) = sender_type {
        sql.push_str(&format!(" AND m.sender_type = ?{idx}"));
        param_values.push(sender_type_val.to_string());
        idx += 1;
    }

    sql.push_str(&format!(" ORDER BY m.seq DESC LIMIT ?{idx}"));
    param_values.push(limit.to_string());

    let mut stmt = conn.prepare(&sql).unwrap();
    let params_refs: Vec<&dyn rusqlite::types::ToSql> = param_values
        .iter()
        .map(|v| v as &dyn rusqlite::types::ToSql)
        .collect();

    let events: Vec<ActivityEvent> = stmt
        .query_map(params_refs.as_slice(), |row| {
            Ok(ActivityEvent {
                event_type: "message".to_string(),
                message_id: row.get(0)?,
                room_id: row.get(1)?,
                room_name: row.get(2)?,
                sender: row.get(3)?,
                sender_type: row.get(4)?,
                content: row.get(5)?,
                created_at: row.get(6)?,
                edited_at: row.get(7)?,
                reply_to: row.get(8)?,
                seq: row.get(9)?,
            })
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    let count = events.len();
    Json(ActivityResponse {
        events,
        count,
        since: since.map(String::from),
    })
}

// --- Typing Indicator ---

/// In-memory dedup: tracks last typing notification per (room, sender) to avoid spam.
/// Key: "room_id:sender", Value: timestamp (seconds since epoch).
pub struct TypingTracker {
    pub last_typing: StdMutex<HashMap<String, u64>>,
}

impl Default for TypingTracker {
    fn default() -> Self {
        Self {
            last_typing: StdMutex::new(HashMap::new()),
        }
    }
}

#[post("/api/v1/rooms/<room_id>/typing", format = "json", data = "<body>")]
pub fn notify_typing(
    db: &State<Db>,
    events: &State<EventBus>,
    typing_tracker: &State<TypingTracker>,
    room_id: &str,
    body: Json<TypingNotification>,
) -> Result<Json<serde_json::Value>, (Status, Json<serde_json::Value>)> {
    let sender = body.sender.trim().to_string();
    if sender.is_empty() || sender.len() > 100 {
        return Err((
            Status::BadRequest,
            Json(serde_json::json!({"error": "Sender must be 1-100 characters"})),
        ));
    }

    // Verify room exists
    let conn = db.conn.lock().unwrap();
    let room_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM rooms WHERE id = ?1",
            params![room_id],
            |r| r.get::<_, i64>(0),
        )
        .map(|c| c > 0)
        .unwrap_or(false);

    if !room_exists {
        return Err((
            Status::NotFound,
            Json(serde_json::json!({"error": "Room not found"})),
        ));
    }
    drop(conn);

    // Dedup: only publish if last typing notification was >2 seconds ago
    let key = format!("{}:{}", room_id, sender);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    {
        let mut tracker = typing_tracker.last_typing.lock().unwrap();
        if let Some(&last) = tracker.get(&key)
            && now - last < 2
        {
            return Ok(Json(serde_json::json!({"ok": true})));
        }
        tracker.insert(key, now);

        // Prune old entries (>30s) to prevent memory leak
        tracker.retain(|_, &mut ts| now - ts < 30);
    }

    events.publish(ChatEvent::Typing {
        sender,
        room_id: room_id.to_string(),
    });

    Ok(Json(serde_json::json!({"ok": true})))
}

// --- SSE Stream ---

#[get("/api/v1/rooms/<room_id>/stream?<since>&<after>")]
pub fn message_stream(
    db: &State<Db>,
    events: &State<EventBus>,
    room_id: &str,
    since: Option<&str>,
    after: Option<i64>,
) -> EventStream![] {
    let mut rx = events.sender.subscribe();
    let room_id = room_id.to_string();

    // Replay missed messages if `after` or `since` provided
    let replay: Vec<Message> = if let Some(after_val) = after {
        // Preferred: cursor-based replay using monotonic seq
        let conn = db.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, room_id, sender, content, metadata, created_at, edited_at, reply_to, sender_type, seq FROM messages WHERE room_id = ?1 AND seq > ?2 ORDER BY seq ASC LIMIT 100",
            )
            .ok();
        if let Some(ref mut s) = stmt {
            s.query_map(params![&room_id, after_val], |row| {
                let metadata_str: String = row.get(4)?;
                Ok(Message {
                    id: row.get(0)?,
                    room_id: row.get(1)?,
                    sender: row.get(2)?,
                    content: row.get(3)?,
                    metadata: serde_json::from_str(&metadata_str)
                        .unwrap_or(serde_json::json!({})),
                    created_at: row.get(5)?,
                    edited_at: row.get(6)?,
                    reply_to: row.get(7)?,
                    sender_type: row.get(8)?,
                    seq: row.get(9)?,
                })
            })
            .ok()
            .map(|rows| rows.filter_map(|r| r.ok()).collect())
            .unwrap_or_default()
        } else {
            vec![]
        }
    } else if let Some(since_val) = since {
        // Backward compat: timestamp-based replay
        let conn = db.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, room_id, sender, content, metadata, created_at, edited_at, reply_to, sender_type, seq FROM messages WHERE room_id = ?1 AND created_at > ?2 ORDER BY seq ASC LIMIT 100",
            )
            .ok();
        if let Some(ref mut s) = stmt {
            s.query_map(params![&room_id, since_val], |row| {
                let metadata_str: String = row.get(4)?;
                Ok(Message {
                    id: row.get(0)?,
                    room_id: row.get(1)?,
                    sender: row.get(2)?,
                    content: row.get(3)?,
                    metadata: serde_json::from_str(&metadata_str)
                        .unwrap_or(serde_json::json!({})),
                    created_at: row.get(5)?,
                    edited_at: row.get(6)?,
                    reply_to: row.get(7)?,
                    sender_type: row.get(8)?,
                    seq: row.get(9)?,
                })
            })
            .ok()
            .map(|rows| rows.filter_map(|r| r.ok()).collect())
            .unwrap_or_default()
        } else {
            vec![]
        }
    } else {
        vec![]
    };

    EventStream! {
        // Send replayed messages first
        for msg in replay {
            yield Event::json(&msg).event("message");
        }

        let mut heartbeat = interval(Duration::from_secs(15));

        loop {
            tokio::select! {
                msg = rx.recv() => {
                    match msg {
                        Ok(ChatEvent::NewMessage(m)) if m.room_id == room_id => {
                            yield Event::json(&m).event("message");
                        }
                        Ok(ChatEvent::MessageEdited(m)) if m.room_id == room_id => {
                            yield Event::json(&m).event("message_edited");
                        }
                        Ok(ChatEvent::MessageDeleted { ref id, room_id: ref rid }) if *rid == room_id => {
                            yield Event::json(&serde_json::json!({"id": id, "room_id": rid})).event("message_deleted");
                        }
                        Ok(ChatEvent::Typing { ref sender, room_id: ref rid }) if *rid == room_id => {
                            yield Event::json(&serde_json::json!({"sender": sender, "room_id": rid})).event("typing");
                        }
                        Ok(ChatEvent::FileUploaded(ref f)) if f.room_id == room_id => {
                            yield Event::json(f).event("file_uploaded");
                        }
                        Ok(ChatEvent::FileDeleted { ref id, room_id: ref rid }) if *rid == room_id => {
                            yield Event::json(&serde_json::json!({"id": id, "room_id": rid})).event("file_deleted");
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                        _ => {} // different room or lagged
                    }
                }
                _ = heartbeat.tick() => {
                    let now = chrono::Utc::now().to_rfc3339();
                    yield Event::json(&serde_json::json!({"time": now})).event("heartbeat");
                }
            }
        }
    }
}

// --- File Attachments ---

/// Max file size: 5MB (after base64 decode)
const MAX_FILE_SIZE: usize = 5 * 1024 * 1024;

#[post("/api/v1/rooms/<room_id>/files", format = "json", data = "<body>")]
pub fn upload_file(
    db: &State<Db>,
    events: &State<EventBus>,
    rate_limiter: &State<RateLimiter>,
    ip: ClientIp,
    room_id: &str,
    body: Json<FileUpload>,
) -> Result<Json<FileInfo>, (Status, Json<serde_json::Value>)> {
    use base64::Engine;

    if !rate_limiter.check(&format!("upload_file:{}", ip.0), 10, 60) {
        return Err((
            Status::TooManyRequests,
            Json(serde_json::json!({"error": "Rate limited: max 10 file uploads per minute"})),
        ));
    }

    let sender = body.sender.trim().to_string();
    let filename = body.filename.trim().to_string();

    if sender.is_empty() || sender.len() > 100 {
        return Err((
            Status::BadRequest,
            Json(serde_json::json!({"error": "Sender must be 1-100 characters"})),
        ));
    }
    if filename.is_empty() || filename.len() > 255 {
        return Err((
            Status::BadRequest,
            Json(serde_json::json!({"error": "Filename must be 1-255 characters"})),
        ));
    }
    if body.data.is_empty() {
        return Err((
            Status::BadRequest,
            Json(serde_json::json!({"error": "File data must not be empty"})),
        ));
    }

    // Decode base64
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(&body.data)
        .map_err(|_| {
            (
                Status::BadRequest,
                Json(serde_json::json!({"error": "Invalid base64 data"})),
            )
        })?;

    if decoded.len() > MAX_FILE_SIZE {
        return Err((
            Status::BadRequest,
            Json(serde_json::json!({"error": format!("File too large: {} bytes (max {} bytes)", decoded.len(), MAX_FILE_SIZE)})),
        ));
    }

    let conn = db.conn.lock().unwrap();

    // Verify room exists
    let room_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM rooms WHERE id = ?1",
            params![room_id],
            |r| r.get::<_, i64>(0),
        )
        .map(|c| c > 0)
        .unwrap_or(false);

    if !room_exists {
        return Err((
            Status::NotFound,
            Json(serde_json::json!({"error": "Room not found"})),
        ));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let size = decoded.len() as i64;

    conn.execute(
        "INSERT INTO files (id, room_id, sender, filename, content_type, size, data, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![&id, room_id, &sender, &filename, &body.content_type, size, &decoded, &now],
    )
    .map_err(|e| {
        (
            Status::InternalServerError,
            Json(serde_json::json!({"error": e.to_string()})),
        )
    })?;

    let file_info = FileInfo {
        id: id.clone(),
        room_id: room_id.to_string(),
        sender,
        filename,
        content_type: body.content_type.clone(),
        size,
        url: format!("/api/v1/files/{}", id),
        created_at: now,
    };

    events.publish(ChatEvent::FileUploaded(file_info.clone()));

    Ok(Json(file_info))
}

#[get("/api/v1/files/<file_id>")]
pub fn download_file(
    db: &State<Db>,
    file_id: &str,
) -> Result<(rocket::http::ContentType, Vec<u8>), (Status, Json<serde_json::Value>)> {
    let conn = db.conn.lock().unwrap();
    conn.query_row(
        "SELECT content_type, data FROM files WHERE id = ?1",
        params![file_id],
        |row| {
            let ct: String = row.get(0)?;
            let data: Vec<u8> = row.get(1)?;
            Ok((ct, data))
        },
    )
    .map(|(ct, data)| {
        let content_type = rocket::http::ContentType::parse_flexible(&ct)
            .unwrap_or(rocket::http::ContentType::Binary);
        (content_type, data)
    })
    .map_err(|_| {
        (
            Status::NotFound,
            Json(serde_json::json!({"error": "File not found"})),
        )
    })
}

#[get("/api/v1/files/<file_id>/info")]
pub fn file_info(
    db: &State<Db>,
    file_id: &str,
) -> Result<Json<FileInfo>, (Status, Json<serde_json::Value>)> {
    let conn = db.conn.lock().unwrap();
    conn.query_row(
        "SELECT id, room_id, sender, filename, content_type, size, created_at FROM files WHERE id = ?1",
        params![file_id],
        |row| {
            let id: String = row.get(0)?;
            Ok(FileInfo {
                id: id.clone(),
                room_id: row.get(1)?,
                sender: row.get(2)?,
                filename: row.get(3)?,
                content_type: row.get(4)?,
                size: row.get(5)?,
                url: format!("/api/v1/files/{}", id),
                created_at: row.get(6)?,
            })
        },
    )
    .map(Json)
    .map_err(|_| {
        (
            Status::NotFound,
            Json(serde_json::json!({"error": "File not found"})),
        )
    })
}

#[get("/api/v1/rooms/<room_id>/files")]
pub fn list_files(
    db: &State<Db>,
    room_id: &str,
) -> Result<Json<Vec<FileInfo>>, (Status, Json<serde_json::Value>)> {
    let conn = db.conn.lock().unwrap();

    // Verify room exists
    let room_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM rooms WHERE id = ?1",
            params![room_id],
            |r| r.get::<_, i64>(0),
        )
        .map(|c| c > 0)
        .unwrap_or(false);

    if !room_exists {
        return Err((
            Status::NotFound,
            Json(serde_json::json!({"error": "Room not found"})),
        ));
    }

    let mut stmt = conn
        .prepare("SELECT id, room_id, sender, filename, content_type, size, created_at FROM files WHERE room_id = ?1 ORDER BY created_at DESC")
        .unwrap();

    let files = stmt
        .query_map(params![room_id], |row| {
            let id: String = row.get(0)?;
            Ok(FileInfo {
                id: id.clone(),
                room_id: row.get(1)?,
                sender: row.get(2)?,
                filename: row.get(3)?,
                content_type: row.get(4)?,
                size: row.get(5)?,
                url: format!("/api/v1/files/{}", id),
                created_at: row.get(6)?,
            })
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    Ok(Json(files))
}

#[delete("/api/v1/rooms/<room_id>/files/<file_id>?<sender>")]
pub fn delete_file(
    db: &State<Db>,
    events: &State<EventBus>,
    room_id: &str,
    file_id: &str,
    sender: Option<&str>,
    admin: Option<AdminKey>,
) -> Result<Json<serde_json::Value>, (Status, Json<serde_json::Value>)> {
    let conn = db.conn.lock().unwrap();

    // Fetch existing file
    let existing_sender: String = conn
        .query_row(
            "SELECT sender FROM files WHERE id = ?1 AND room_id = ?2",
            params![file_id, room_id],
            |r| r.get(0),
        )
        .map_err(|_| {
            (
                Status::NotFound,
                Json(serde_json::json!({"error": "File not found"})),
            )
        })?;

    // Check if admin key matches the room's admin key
    let is_room_admin = if let Some(ref admin_key) = admin {
        let stored_key: Option<String> = conn
            .query_row(
                "SELECT admin_key FROM rooms WHERE id = ?1",
                params![room_id],
                |r| r.get(0),
            )
            .ok()
            .flatten();
        stored_key.as_deref() == Some(&admin_key.0)
    } else {
        false
    };

    // Room admin can delete any file; otherwise sender must match
    if !is_room_admin {
        let sender = sender.ok_or_else(|| {
            (
                Status::BadRequest,
                Json(serde_json::json!({"error": "sender query parameter required (or use room admin key)"})),
            )
        })?;
        if sender != existing_sender {
            return Err((
                Status::Forbidden,
                Json(serde_json::json!({"error": "Only the original uploader can delete this file"})),
            ));
        }
    }

    conn.execute(
        "DELETE FROM files WHERE id = ?1 AND room_id = ?2",
        params![file_id, room_id],
    )
    .map_err(|e| {
        (
            Status::InternalServerError,
            Json(serde_json::json!({"error": e.to_string()})),
        )
    })?;

    events.publish(ChatEvent::FileDeleted {
        id: file_id.to_string(),
        room_id: room_id.to_string(),
    });

    Ok(Json(serde_json::json!({"deleted": true})))
}

// --- llms.txt ---

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
- GET /api/v1/rooms/{id}/messages?after=<seq>&since=&limit=&before=&sender=&sender_type= — poll messages. Use `after=<seq>` for reliable cursor-based pagination (preferred). `since=` (timestamp) kept for backward compat. Each message has a monotonic `seq` integer.
- GET /api/v1/rooms/{id}/stream?after=<seq>&since= — SSE real-time stream. Use `after=<seq>` to replay missed messages by cursor (preferred over `since=`). Events: message, message_edited, message_deleted, typing

## Typing Indicators
- POST /api/v1/rooms/{id}/typing — notify typing (body: {"sender": "..."}). Ephemeral, not stored. Deduped server-side (2s per sender).

## Activity Feed
- GET /api/v1/activity?after=<seq>&since=&limit=&room_id=&sender=&sender_type= — cross-room activity feed (newest first). Use `after=<seq>` for cursor-based pagination (preferred). Returns all messages across rooms. Each event includes a `seq` field for cursor tracking.

## Files / Attachments
- POST /api/v1/rooms/{id}/files — upload file (body: {"sender": "...", "filename": "...", "content_type": "image/png", "data": "<base64>"})
- GET /api/v1/rooms/{id}/files — list files in room (metadata only, no binary data)
- GET /api/v1/files/{file_id} — download file (raw binary with correct Content-Type)
- GET /api/v1/files/{file_id}/info — file metadata (id, sender, filename, size, url, created_at)
- DELETE /api/v1/rooms/{id}/files/{file_id}?sender=... — delete file (sender must match, or use room admin key)
- Max file size: 5MB. Data must be base64-encoded in the upload request.
- SSE events: file_uploaded, file_deleted (same stream as messages)

## System
- GET /api/v1/health — health check
- GET /api/v1/stats — global stats (includes by_sender_type breakdown and active_by_type_1h)
"#;

// --- OpenAPI ---

#[get("/api/v1/openapi.json")]
pub fn openapi_json() -> (rocket::http::ContentType, &'static str) {
    (rocket::http::ContentType::JSON, include_str!("../openapi.json"))
}

// --- 429 catcher ---

#[rocket::catch(429)]
pub fn too_many_requests() -> Json<serde_json::Value> {
    Json(serde_json::json!({"error": "Too many requests"}))
}

#[rocket::catch(404)]
pub fn not_found() -> Json<serde_json::Value> {
    Json(serde_json::json!({"error": "Not found"}))
}

// --- SPA Fallback ---

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

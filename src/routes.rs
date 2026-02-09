use crate::db::Db;
use crate::events::{ChatEvent, EventBus};
use crate::models::*;
use crate::rate_limit::RateLimiter;
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome, Request};
use rocket::response::stream::{Event, EventStream};
use rocket::serde::json::Json;
use rocket::{State, get, post, delete};
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
    Json(serde_json::json!({
        "rooms": room_count,
        "messages": message_count,
        "active_senders_1h": active_senders
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
    let conn = db.conn.lock().unwrap();

    match conn.execute(
        "INSERT INTO rooms (id, name, description, created_by, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![&id, &name, &body.description, &body.created_by, &now, &now],
    ) {
        Ok(_) => Ok(Json(serde_json::json!({
            "id": id,
            "name": name,
            "description": body.description,
            "created_by": body.created_by,
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
    _admin: AdminKey,
) -> Result<Json<serde_json::Value>, (Status, Json<serde_json::Value>)> {
    let conn = db.conn.lock().unwrap();
    let deleted = conn
        .execute("DELETE FROM rooms WHERE id = ?1", params![room_id])
        .unwrap_or(0);
    if deleted > 0 {
        Ok(Json(serde_json::json!({"deleted": true})))
    } else {
        Err((
            Status::NotFound,
            Json(serde_json::json!({"error": "Room not found"})),
        ))
    }
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

    conn.execute(
        "INSERT INTO messages (id, room_id, sender, content, metadata, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![&id, room_id, &sender, &content, serde_json::to_string(&metadata).unwrap(), &now],
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
    };

    // Publish event for SSE
    events.publish(ChatEvent::NewMessage(msg.clone()));

    Ok(Json(msg))
}

#[get("/api/v1/rooms/<room_id>/messages?<since>&<limit>&<before>&<sender>")]
pub fn get_messages(
    db: &State<Db>,
    room_id: &str,
    since: Option<&str>,
    limit: Option<i64>,
    before: Option<&str>,
    sender: Option<&str>,
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

    let mut sql = String::from("SELECT id, room_id, sender, content, metadata, created_at FROM messages WHERE room_id = ?1");
    let mut param_values: Vec<String> = vec![room_id.to_string()];
    let mut idx = 2;

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

    sql.push_str(&format!(" ORDER BY created_at ASC LIMIT ?{idx}"));
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

// --- SSE Stream ---

#[get("/api/v1/rooms/<room_id>/stream?<since>")]
pub fn message_stream(
    db: &State<Db>,
    events: &State<EventBus>,
    room_id: &str,
    since: Option<&str>,
) -> EventStream![] {
    let mut rx = events.sender.subscribe();
    let room_id = room_id.to_string();

    // Replay missed messages if `since` provided
    let replay: Vec<Message> = if let Some(since_val) = since {
        let conn = db.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, room_id, sender, content, metadata, created_at FROM messages WHERE room_id = ?1 AND created_at > ?2 ORDER BY created_at ASC LIMIT 100",
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
- No auth required. Identity is self-declared via the `sender` field.
- Trust-based: designed for private LAN usage.
- Admin key (Bearer token) required only for room deletion.

## Rooms
- POST /api/v1/rooms — create room (body: {"name": "...", "description": "..."})
- GET /api/v1/rooms — list all rooms with stats
- GET /api/v1/rooms/{id} — room details
- DELETE /api/v1/rooms/{id} — delete room (admin auth required)

## Messages
- POST /api/v1/rooms/{id}/messages — send message (body: {"sender": "...", "content": "..."})
- GET /api/v1/rooms/{id}/messages?since=&limit=&before=&sender= — poll messages
- GET /api/v1/rooms/{id}/stream?since= — SSE real-time stream

## System
- GET /api/v1/health — health check
- GET /api/v1/stats — global stats
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

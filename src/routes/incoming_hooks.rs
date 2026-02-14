use crate::db::{self, Db};
use crate::events::{ChatEvent, EventBus};
use crate::models::*;
use crate::rate_limit::RateLimiter;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{delete, get, post, put, State};
use rusqlite::params;

use super::{AdminKey, ClientIp};

/// Create an incoming webhook for a room (admin key required).
#[post(
    "/api/v1/rooms/<room_id>/incoming-webhooks",
    format = "json",
    data = "<body>"
)]
pub fn create_incoming_webhook(
    db: &State<Db>,
    room_id: &str,
    admin: AdminKey,
    body: Json<CreateIncomingWebhook>,
) -> Result<Json<IncomingWebhook>, (Status, Json<serde_json::Value>)> {
    let conn = db.conn.lock().unwrap();

    // Verify room exists and admin key matches
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

    match stored_key {
        Some(ref key) if key == &admin.0 => {}
        _ => {
            return Err((
                Status::Forbidden,
                Json(serde_json::json!({"error": "Invalid admin key for this room"})),
            ));
        }
    }

    let name = body.name.trim().to_string();
    if name.is_empty() || name.len() > 100 {
        return Err((
            Status::BadRequest,
            Json(serde_json::json!({"error": "Name must be 1-100 characters"})),
        ));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let token = db::generate_webhook_token();
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO incoming_webhooks (id, room_id, name, token, created_by, created_at, active) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1)",
        params![&id, room_id, &name, &token, &body.created_by, &now],
    )
    .map_err(|e| {
        (
            Status::InternalServerError,
            Json(serde_json::json!({"error": e.to_string()})),
        )
    })?;

    Ok(Json(IncomingWebhook {
        id,
        room_id: room_id.to_string(),
        name,
        token: token.clone(),
        created_by: body.created_by.clone(),
        created_at: now,
        active: true,
        url: Some(format!("/api/v1/hook/{}", token)),
    }))
}

/// List incoming webhooks for a room (admin key required).
#[get("/api/v1/rooms/<room_id>/incoming-webhooks")]
pub fn list_incoming_webhooks(
    db: &State<Db>,
    room_id: &str,
    admin: AdminKey,
) -> Result<Json<Vec<IncomingWebhook>>, (Status, Json<serde_json::Value>)> {
    let conn = db.conn.lock().unwrap();

    // Verify room exists and admin key matches
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

    match stored_key {
        Some(ref key) if key == &admin.0 => {}
        _ => {
            return Err((
                Status::Forbidden,
                Json(serde_json::json!({"error": "Invalid admin key for this room"})),
            ));
        }
    }

    let mut stmt = conn
        .prepare(
            "SELECT id, room_id, name, token, created_by, created_at, active FROM incoming_webhooks WHERE room_id = ?1 ORDER BY created_at DESC",
        )
        .unwrap();

    let hooks: Vec<IncomingWebhook> = stmt
        .query_map(params![room_id], |row| {
            let token: String = row.get(3)?;
            Ok(IncomingWebhook {
                id: row.get(0)?,
                room_id: row.get(1)?,
                name: row.get(2)?,
                token: token.clone(),
                created_by: row.get(4)?,
                created_at: row.get(5)?,
                active: row.get::<_, i32>(6)? != 0,
                url: Some(format!("/api/v1/hook/{}", token)),
            })
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    Ok(Json(hooks))
}

/// Update an incoming webhook (admin key required).
#[put(
    "/api/v1/rooms/<room_id>/incoming-webhooks/<webhook_id>",
    format = "json",
    data = "<body>"
)]
pub fn update_incoming_webhook(
    db: &State<Db>,
    room_id: &str,
    webhook_id: &str,
    admin: AdminKey,
    body: Json<UpdateIncomingWebhook>,
) -> Result<Json<serde_json::Value>, (Status, Json<serde_json::Value>)> {
    let conn = db.conn.lock().unwrap();

    // Verify room exists and admin key matches
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

    match stored_key {
        Some(ref key) if key == &admin.0 => {}
        _ => {
            return Err((
                Status::Forbidden,
                Json(serde_json::json!({"error": "Invalid admin key for this room"})),
            ));
        }
    }

    // Verify webhook exists in this room
    let exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM incoming_webhooks WHERE id = ?1 AND room_id = ?2",
            params![webhook_id, room_id],
            |r| r.get::<_, i64>(0),
        )
        .unwrap_or(0)
        > 0;

    if !exists {
        return Err((
            Status::NotFound,
            Json(serde_json::json!({"error": "Incoming webhook not found"})),
        ));
    }

    let mut updates: Vec<String> = Vec::new();
    let mut values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    let mut idx = 1;

    if let Some(ref name) = body.name {
        let name = name.trim();
        if name.is_empty() || name.len() > 100 {
            return Err((
                Status::BadRequest,
                Json(serde_json::json!({"error": "Name must be 1-100 characters"})),
            ));
        }
        updates.push(format!("name = ?{}", idx));
        values.push(Box::new(name.to_string()));
        idx += 1;
    }
    if let Some(active) = body.active {
        updates.push(format!("active = ?{}", idx));
        values.push(Box::new(active as i32));
        idx += 1;
    }

    if updates.is_empty() {
        return Err((
            Status::BadRequest,
            Json(serde_json::json!({"error": "No fields to update"})),
        ));
    }

    let sql = format!(
        "UPDATE incoming_webhooks SET {} WHERE id = ?{} AND room_id = ?{}",
        updates.join(", "),
        idx,
        idx + 1
    );
    values.push(Box::new(webhook_id.to_string()));
    values.push(Box::new(room_id.to_string()));

    let param_refs: Vec<&dyn rusqlite::ToSql> = values.iter().map(|v| v.as_ref()).collect();
    conn.execute(&sql, param_refs.as_slice()).map_err(|e| {
        (
            Status::InternalServerError,
            Json(serde_json::json!({"error": e.to_string()})),
        )
    })?;

    Ok(Json(
        serde_json::json!({"updated": true, "id": webhook_id}),
    ))
}

/// Delete an incoming webhook (admin key required).
#[delete("/api/v1/rooms/<room_id>/incoming-webhooks/<webhook_id>")]
pub fn delete_incoming_webhook(
    db: &State<Db>,
    room_id: &str,
    webhook_id: &str,
    admin: AdminKey,
) -> Result<Json<serde_json::Value>, (Status, Json<serde_json::Value>)> {
    let conn = db.conn.lock().unwrap();

    // Verify room exists and admin key matches
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

    match stored_key {
        Some(ref key) if key == &admin.0 => {}
        _ => {
            return Err((
                Status::Forbidden,
                Json(serde_json::json!({"error": "Invalid admin key for this room"})),
            ));
        }
    }

    let deleted = conn
        .execute(
            "DELETE FROM incoming_webhooks WHERE id = ?1 AND room_id = ?2",
            params![webhook_id, room_id],
        )
        .unwrap_or(0);

    if deleted == 0 {
        return Err((
            Status::NotFound,
            Json(serde_json::json!({"error": "Incoming webhook not found"})),
        ));
    }

    Ok(Json(
        serde_json::json!({"deleted": true, "id": webhook_id}),
    ))
}

/// Post a message via incoming webhook token. No auth needed — the token IS the auth.
#[post("/api/v1/hook/<token>", format = "json", data = "<body>")]
pub fn post_via_hook(
    db: &State<Db>,
    events: &State<EventBus>,
    rate_limiter: &State<RateLimiter>,
    _ip: ClientIp,
    token: &str,
    body: Json<IncomingWebhookMessage>,
) -> Result<Json<Message>, (Status, Json<serde_json::Value>)> {
    // Rate limit per token (60/min, same as regular messages)
    if !rate_limiter.check(&format!("hook:{}", token), 60, 60) {
        return Err((
            Status::TooManyRequests,
            Json(serde_json::json!({"error": "Rate limited: max 60 messages per minute per webhook"})),
        ));
    }

    let conn = db.conn.lock().unwrap();

    // Look up the webhook by token
    let hook: (String, String, String, i32) = conn
        .query_row(
            "SELECT id, room_id, name, active FROM incoming_webhooks WHERE token = ?1",
            params![token],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .map_err(|_| {
            (
                Status::NotFound,
                Json(serde_json::json!({"error": "Invalid webhook token"})),
            )
        })?;

    let (_hook_id, room_id, hook_name, active) = hook;

    if active == 0 {
        return Err((
            Status::Forbidden,
            Json(serde_json::json!({"error": "This incoming webhook is disabled"})),
        ));
    }

    // Verify room still exists
    let room_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM rooms WHERE id = ?1",
            params![&room_id],
            |r| r.get::<_, i64>(0),
        )
        .map(|c| c > 0)
        .unwrap_or(false);

    if !room_exists {
        return Err((
            Status::NotFound,
            Json(serde_json::json!({"error": "Room no longer exists"})),
        ));
    }

    let content = body.content.trim().to_string();
    if content.is_empty() || content.len() > 10_000 {
        return Err((
            Status::BadRequest,
            Json(serde_json::json!({"error": "Content must be 1-10000 characters"})),
        ));
    }

    // Use provided sender or fall back to webhook name
    let sender = body
        .sender
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty() && s.len() <= 100)
        .unwrap_or(&hook_name)
        .to_string();

    let sender_type = body.sender_type.clone().or(Some("agent".to_string()));
    let metadata = body.metadata.clone().unwrap_or(serde_json::json!({}));

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    // Compute next monotonic seq
    let seq: i64 = conn
        .query_row("SELECT COALESCE(MAX(seq), 0) + 1 FROM messages", [], |r| {
            r.get(0)
        })
        .unwrap_or(1);

    conn.execute(
        "INSERT INTO messages (id, room_id, sender, content, metadata, created_at, sender_type, seq) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![&id, &room_id, &sender, &content, serde_json::to_string(&metadata).unwrap(), &now, &sender_type, seq],
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
        params![&now, &room_id],
    )
    .ok();

    // Index in FTS
    crate::db::upsert_fts(&conn, &id);

    let msg = Message {
        id,
        room_id: room_id.clone(),
        sender,
        content,
        metadata,
        created_at: now,
        edited_at: None,
        reply_to: None,
        sender_type,
        seq,
        pinned_at: None,
        pinned_by: None,
    };

    // Publish event for SSE and outgoing webhooks
    events.publish(ChatEvent::NewMessage(msg.clone()));

    Ok(Json(msg))
}

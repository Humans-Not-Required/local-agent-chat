use crate::db::Db;
use crate::events::{ChatEvent, EventBus};
use crate::models::*;
use crate::rate_limit::{RateLimitConfig, RateLimiter};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{delete, get, post, put, State};
use rusqlite::params;

use super::{AdminKey, ClientIp};

#[post("/api/v1/rooms/<room_id>/messages", format = "json", data = "<body>")]
pub fn send_message(
    db: &State<Db>,
    events: &State<EventBus>,
    rate_limiter: &State<RateLimiter>,
    rate_config: &State<RateLimitConfig>,
    ip: ClientIp,
    room_id: &str,
    body: Json<SendMessage>,
) -> Result<crate::rate_limit::RateLimited<Message>, (Status, Json<serde_json::Value>)> {
    let rl = rate_limiter.check_with_info(&format!("send_msg:{}", ip.0), rate_config.messages_max, rate_config.messages_window_secs);
    if !rl.allowed {
        return Err((
            Status::TooManyRequests,
            Json(serde_json::json!({
                "error": format!("Rate limited: max {} messages per minute", rate_config.messages_max),
                "retry_after_secs": rl.retry_after_secs,
                "limit": rl.limit,
                "remaining": 0
            })),
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

    let conn = db.conn();

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
    let reply_to = body
        .reply_to
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(String::from);

    // Resolve sender_type: top-level field takes priority, fall back to metadata.sender_type
    let sender_type = body.sender_type.clone().or_else(|| {
        metadata
            .get("sender_type")
            .and_then(|v| v.as_str())
            .map(String::from)
    });

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
                Json(serde_json::json!({"error": "Referenced reply_to message not found in this room"})),
            ));
        }
    }

    // Compute next monotonic seq
    let seq: i64 = conn
        .query_row("SELECT COALESCE(MAX(seq), 0) + 1 FROM messages", [], |r| {
            r.get(0)
        })
        .unwrap_or(1);

    conn.execute(
        "INSERT INTO messages (id, room_id, sender, content, metadata, created_at, reply_to, sender_type, seq) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![&id, room_id, &sender, &content, serde_json::to_string(&metadata).unwrap_or_default(), &now, &reply_to, &sender_type, seq],
    )
    .map_err(|_e| {
        (
            Status::InternalServerError,
            Json(serde_json::json!({"error": "Internal server error"})),
        )
    })?;

    // Update room's updated_at
    conn.execute(
        "UPDATE rooms SET updated_at = ?1 WHERE id = ?2",
        params![&now, room_id],
    )
    .ok();

    // Update FTS index
    crate::db::upsert_fts(&conn, &id);

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
        pinned_at: None,
        pinned_by: None,
        edit_count: 0,
    };

    // Publish event for SSE
    events.publish(ChatEvent::NewMessage(msg.clone()));

    Ok(crate::rate_limit::RateLimited::new(Json(msg), rl))
}

#[put(
    "/api/v1/rooms/<room_id>/messages/<message_id>",
    format = "json",
    data = "<body>"
)]
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

    let conn = db.conn();

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

    // Save previous content to edit history
    let previous_content: String = conn
        .query_row(
            "SELECT content FROM messages WHERE id = ?1",
            params![message_id],
            |r| r.get(0),
        )
        .unwrap_or_default();

    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO message_edits (id, message_id, previous_content, edited_at, editor) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![uuid::Uuid::new_v4().to_string(), message_id, &previous_content, &now, &sender],
    ).ok();

    let metadata = body.metadata.clone();

    // Update content and edited_at; optionally update metadata
    if let Some(ref meta) = metadata {
        conn.execute(
            "UPDATE messages SET content = ?1, metadata = ?2, edited_at = ?3 WHERE id = ?4",
            params![
                &content,
                serde_json::to_string(meta).unwrap_or_default(),
                &now,
                message_id
            ],
        )
        .map_err(|_e| {
            (
                Status::InternalServerError,
                Json(serde_json::json!({"error": "Internal server error"})),
            )
        })?;
    } else {
        conn.execute(
            "UPDATE messages SET content = ?1, edited_at = ?2 WHERE id = ?3",
            params![&content, &now, message_id],
        )
        .map_err(|_e| {
            (
                Status::InternalServerError,
                Json(serde_json::json!({"error": "Internal server error"})),
            )
        })?;
    }

    // Fetch the updated message
    let msg = conn
        .query_row(
            "SELECT m.id, m.room_id, m.sender, m.content, m.metadata, m.created_at, m.edited_at, m.reply_to, m.sender_type, m.seq, m.pinned_at, m.pinned_by, \
             (SELECT COUNT(*) FROM message_edits WHERE message_id = m.id) \
             FROM messages m WHERE m.id = ?1",
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
                    pinned_at: row.get(10)?,
                    pinned_by: row.get(11)?,
                    edit_count: row.get(12)?,
                })
            },
        )
        .map_err(|_e| {
            (
                Status::InternalServerError,
                Json(serde_json::json!({"error": "Internal server error"})),
            )
        })?;

    // Update FTS index
    crate::db::upsert_fts(&conn, message_id);

    events.publish(ChatEvent::MessageEdited(msg.clone()));

    Ok(Json(msg))
}

#[delete("/api/v1/rooms/<room_id>/messages/<message_id>?<sender>")]
pub fn delete_message(
    db: &State<Db>,
    events: &State<EventBus>,
    room_id: &str,
    message_id: &str,
    sender: Option<&str>,
    admin: Option<AdminKey>,
) -> Result<Json<serde_json::Value>, (Status, Json<serde_json::Value>)> {
    let conn = db.conn();

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
                Json(serde_json::json!({"error": "Sender query parameter required (or use room admin key)"})),
            )
        })?;
        if sender != existing_sender {
            return Err((
                Status::Forbidden,
                Json(
                    serde_json::json!({"error": "Only the original sender can delete this message"}),
                ),
            ));
        }
    }

    // Remove from FTS index before deleting
    crate::db::delete_fts(&conn, message_id);

    conn.execute(
        "DELETE FROM messages WHERE id = ?1 AND room_id = ?2",
        params![message_id, room_id],
    )
    .map_err(|_e| {
        (
            Status::InternalServerError,
            Json(serde_json::json!({"error": "Internal server error"})),
        )
    })?;

    events.publish(ChatEvent::MessageDeleted {
        id: message_id.to_string(),
        room_id: room_id.to_string(),
    });

    Ok(Json(serde_json::json!({"deleted": true})))
}

#[get(
    "/api/v1/rooms/<room_id>/messages?<since>&<limit>&<before>&<sender>&<sender_type>&<after>&<exclude_sender>&<before_seq>&<latest>"
)]
#[allow(clippy::too_many_arguments)]
pub fn get_messages(
    db: &State<Db>,
    room_id: &str,
    since: Option<&str>,
    limit: Option<i64>,
    before: Option<&str>,
    sender: Option<&str>,
    sender_type: Option<&str>,
    after: Option<i64>,
    exclude_sender: Option<&str>,
    before_seq: Option<i64>,
    latest: Option<i64>,
) -> Result<Json<Vec<Message>>, (Status, Json<serde_json::Value>)> {
    // ?latest=N is a convenience param: returns the N most recent messages in
    // chronological order. Equivalent to before_seq=i64::MAX&limit=N.
    // If before_seq or after is also set, ?latest is ignored (explicit wins).
    let (before_seq, limit) = if latest.is_some() && before_seq.is_none() && after.is_none() {
        let n = latest.unwrap().clamp(1, 500);
        (Some(i64::MAX), Some(n))
    } else {
        (before_seq, limit)
    };

    let conn = db.conn();

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

    let mut sql = String::from(
        "SELECT id, room_id, sender, content, metadata, created_at, edited_at, reply_to, sender_type, seq, pinned_at, pinned_by, \
         (SELECT COUNT(*) FROM message_edits WHERE message_id = messages.id) FROM messages WHERE room_id = ?1",
    );
    let mut param_values: Vec<String> = vec![room_id.to_string()];
    let mut idx = 2;

    if let Some(after_val) = after {
        sql.push_str(&format!(" AND seq > ?{idx}"));
        param_values.push(after_val.to_string());
        idx += 1;
    }
    if let Some(before_seq_val) = before_seq {
        sql.push_str(&format!(" AND seq < ?{idx}"));
        param_values.push(before_seq_val.to_string());
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
    if let Some(exclude_val) = exclude_sender {
        // Support comma-separated list: ?exclude_sender=Forge,Drift,Lux
        let excluded: Vec<&str> = exclude_val
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        if !excluded.is_empty() {
            let placeholders: Vec<String> = excluded
                .iter()
                .enumerate()
                .map(|(i, _)| format!("?{}", idx + i))
                .collect();
            sql.push_str(&format!(" AND sender NOT IN ({})", placeholders.join(",")));
            for name in &excluded {
                param_values.push(name.to_string());
            }
            idx += excluded.len();
        }
    }

    // When using before_seq without after, we want the most recent N messages
    // before that seq. Use DESC ordering and reverse the results.
    let use_desc = before_seq.is_some() && after.is_none();
    if use_desc {
        sql.push_str(&format!(" ORDER BY seq DESC LIMIT ?{idx}"));
    } else {
        sql.push_str(&format!(" ORDER BY seq ASC LIMIT ?{idx}"));
    }
    param_values.push(limit.to_string());

    let mut stmt = conn.prepare(&sql).map_err(|_e| {
        (
            Status::InternalServerError,
            Json(serde_json::json!({"error": "Internal server error"})),
        )
    })?;

    let params_refs: Vec<&dyn rusqlite::types::ToSql> = param_values
        .iter()
        .map(|v| v as &dyn rusqlite::types::ToSql)
        .collect();

    let mut messages: Vec<Message> = stmt
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
                pinned_at: row.get(10)?,
                pinned_by: row.get(11)?,
                edit_count: row.get(12)?,
            })
        })
        .map_err(|_e| {
            (
                Status::InternalServerError,
                Json(serde_json::json!({"error": "Internal server error"})),
            )
        })?
        .filter_map(|r| r.ok())
        .collect();

    // Reverse DESC results to return in chronological order
    if use_desc {
        messages.reverse();
    }

    Ok(Json(messages))
}

#[get("/api/v1/rooms/<room_id>/messages/<message_id>/edits")]
pub fn get_edit_history(
    db: &State<Db>,
    room_id: &str,
    message_id: &str,
) -> Result<Json<EditHistoryResponse>, (Status, Json<serde_json::Value>)> {
    let conn = db.conn();

    // Verify the message exists in this room and get current content
    let current_content: String = conn
        .query_row(
            "SELECT content FROM messages WHERE id = ?1 AND room_id = ?2",
            params![message_id, room_id],
            |r| r.get(0),
        )
        .map_err(|_| {
            (
                Status::NotFound,
                Json(serde_json::json!({"error": "Message not found in this room"})),
            )
        })?;

    // Fetch edit history (chronological: oldest edit first)
    let mut stmt = conn
        .prepare(
            "SELECT id, message_id, previous_content, edited_at, editor \
             FROM message_edits WHERE message_id = ?1 ORDER BY edited_at ASC",
        )
        .map_err(|_| {
            (
                Status::InternalServerError,
                Json(serde_json::json!({"error": "Internal server error"})),
            )
        })?;

    let edits: Vec<MessageEdit> = match stmt.query_map(params![message_id], |row| {
        Ok(MessageEdit {
            id: row.get(0)?,
            message_id: row.get(1)?,
            previous_content: row.get(2)?,
            edited_at: row.get(3)?,
            editor: row.get(4)?,
        })
    }) {
        Ok(rows) => rows.filter_map(|r| r.ok()).collect(),
        Err(_) => Vec::new(),
    };

    let edit_count = edits.len() as i64;

    Ok(Json(EditHistoryResponse {
        message_id: message_id.to_string(),
        current_content,
        edits,
        edit_count,
    }))
}

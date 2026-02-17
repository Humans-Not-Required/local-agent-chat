use crate::db::Db;
use crate::events::{ChatEvent, EventBus};
use crate::models::PinnedMessage;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{delete, get, post, State};
use rusqlite::params;

use super::AdminKey;

#[post("/api/v1/rooms/<room_id>/messages/<message_id>/pin")]
pub fn pin_message(
    db: &State<Db>,
    events: &State<EventBus>,
    room_id: &str,
    message_id: &str,
    admin: AdminKey,
) -> Result<Json<PinnedMessage>, (Status, Json<serde_json::Value>)> {
    let conn = db.conn();

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

    // Verify message exists in this room
    let msg_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM messages WHERE id = ?1 AND room_id = ?2",
            params![message_id, room_id],
            |r| r.get::<_, i64>(0),
        )
        .unwrap_or(0)
        > 0;
    if !msg_exists {
        return Err((
            Status::NotFound,
            Json(serde_json::json!({"error": "Message not found in this room"})),
        ));
    }

    // Check if already pinned
    let already_pinned: bool = conn
        .query_row(
            "SELECT pinned_at FROM messages WHERE id = ?1",
            params![message_id],
            |r| r.get::<_, Option<String>>(0),
        )
        .unwrap_or(None)
        .is_some();
    if already_pinned {
        return Err((
            Status::Conflict,
            Json(serde_json::json!({"error": "Message is already pinned"})),
        ));
    }

    let now = chrono::Utc::now().to_rfc3339();

    // Pin the message
    conn.execute(
        "UPDATE messages SET pinned_at = ?1, pinned_by = ?2 WHERE id = ?3",
        params![&now, "admin", message_id],
    )
    .map_err(|e| {
        (
            Status::InternalServerError,
            Json(serde_json::json!({"error": format!("Failed to pin message: {e}")})),
        )
    })?;

    // Fetch the pinned message
    let pinned = conn
        .query_row(
            "SELECT id, room_id, sender, content, metadata, created_at, edited_at, reply_to, sender_type, seq, pinned_at, pinned_by FROM messages WHERE id = ?1",
            params![message_id],
            |row| {
                let metadata_str: String = row.get(4)?;
                Ok(PinnedMessage {
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
                    pinned_at: row.get::<_, String>(10)?,
                    pinned_by: row.get::<_, String>(11)?,
                })
            },
        )
        .map_err(|_| {
            (
                Status::InternalServerError,
                Json(serde_json::json!({"error": "Failed to fetch pinned message"})),
            )
        })?;

    events.publish(ChatEvent::MessagePinned(pinned.clone()));

    Ok(Json(pinned))
}

#[delete("/api/v1/rooms/<room_id>/messages/<message_id>/pin")]
pub fn unpin_message(
    db: &State<Db>,
    events: &State<EventBus>,
    room_id: &str,
    message_id: &str,
    admin: AdminKey,
) -> Result<Json<serde_json::Value>, (Status, Json<serde_json::Value>)> {
    let conn = db.conn();

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

    // Verify message exists in this room and is pinned
    let is_pinned: bool = conn
        .query_row(
            "SELECT pinned_at FROM messages WHERE id = ?1 AND room_id = ?2",
            params![message_id, room_id],
            |r| r.get::<_, Option<String>>(0),
        )
        .map_err(|_| {
            (
                Status::NotFound,
                Json(serde_json::json!({"error": "Message not found in this room"})),
            )
        })?
        .is_some();

    if !is_pinned {
        return Err((
            Status::BadRequest,
            Json(serde_json::json!({"error": "Message is not pinned"})),
        ));
    }

    // Unpin the message
    conn.execute(
        "UPDATE messages SET pinned_at = NULL, pinned_by = NULL WHERE id = ?1",
        params![message_id],
    )
    .map_err(|e| {
        (
            Status::InternalServerError,
            Json(serde_json::json!({"error": format!("Failed to unpin message: {e}")})),
        )
    })?;

    events.publish(ChatEvent::MessageUnpinned {
        id: message_id.to_string(),
        room_id: room_id.to_string(),
    });

    Ok(Json(serde_json::json!({
        "status": "unpinned",
        "message_id": message_id,
        "room_id": room_id
    })))
}

#[get("/api/v1/rooms/<room_id>/pins")]
pub fn list_pins(
    db: &State<Db>,
    room_id: &str,
) -> Result<Json<Vec<PinnedMessage>>, (Status, Json<serde_json::Value>)> {
    let conn = db.conn();

    // Verify room exists
    let room_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM rooms WHERE id = ?1",
            params![room_id],
            |r| r.get::<_, i64>(0),
        )
        .unwrap_or(0)
        > 0;
    if !room_exists {
        return Err((
            Status::NotFound,
            Json(serde_json::json!({"error": "Room not found"})),
        ));
    }

    let mut stmt = conn
        .prepare(
            "SELECT id, room_id, sender, content, metadata, created_at, edited_at, reply_to, sender_type, seq, pinned_at, pinned_by FROM messages WHERE room_id = ?1 AND pinned_at IS NOT NULL ORDER BY pinned_at DESC",
        )
        .map_err(|e| {
            (
                Status::InternalServerError,
                Json(serde_json::json!({"error": format!("Query error: {e}")})),
            )
        })?;

    let pins: Vec<PinnedMessage> = stmt
        .query_map(params![room_id], |row| {
            let metadata_str: String = row.get(4)?;
            Ok(PinnedMessage {
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
                pinned_at: row.get::<_, String>(10)?,
                pinned_by: row.get::<_, String>(11)?,
            })
        })
        .map_err(|e| {
            (
                Status::InternalServerError,
                Json(serde_json::json!({"error": format!("Query error: {e}")})),
            )
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(Json(pins))
}

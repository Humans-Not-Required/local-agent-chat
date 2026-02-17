use crate::db::Db;
use crate::events::{ChatEvent, EventBus};
use crate::models::*;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{delete, get, put, State};
use rusqlite::params;

/// PUT /api/v1/rooms/<room_id>/bookmark — Add a bookmark
#[put("/api/v1/rooms/<room_id>/bookmark", format = "json", data = "<body>")]
pub fn add_bookmark(
    db: &State<Db>,
    events: &State<EventBus>,
    room_id: &str,
    body: Json<BookmarkAction>,
) -> Result<Json<serde_json::Value>, (Status, Json<serde_json::Value>)> {
    let sender = body.sender.trim();
    if sender.is_empty() || sender.len() > 100 {
        return Err((
            Status::BadRequest,
            Json(serde_json::json!({"error": "Sender must be 1-100 characters"})),
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
        .unwrap_or(0)
        > 0;

    if !room_exists {
        return Err((
            Status::NotFound,
            Json(serde_json::json!({"error": "Room not found"})),
        ));
    }

    let now = chrono::Utc::now().to_rfc3339();

    // INSERT OR IGNORE — idempotent
    let rows = conn
        .execute(
            "INSERT OR IGNORE INTO bookmarks (room_id, sender, created_at) VALUES (?1, ?2, ?3)",
            params![room_id, sender, &now],
        )
        .map_err(|e| {
            (
                Status::InternalServerError,
                Json(serde_json::json!({"error": format!("Database error: {e}")})),
            )
        })?;

    let created = rows > 0;

    if created {
        events.publish(ChatEvent::RoomBookmarked {
            room_id: room_id.to_string(),
            sender: sender.to_string(),
        });
    }

    Ok(Json(serde_json::json!({
        "room_id": room_id,
        "sender": sender,
        "bookmarked": true,
        "created": created
    })))
}

/// DELETE /api/v1/rooms/<room_id>/bookmark?sender=<sender> — Remove a bookmark
#[delete("/api/v1/rooms/<room_id>/bookmark?<sender>")]
pub fn remove_bookmark(
    db: &State<Db>,
    events: &State<EventBus>,
    room_id: &str,
    sender: &str,
) -> Result<Json<serde_json::Value>, (Status, Json<serde_json::Value>)> {
    let sender = sender.trim();
    if sender.is_empty() {
        return Err((
            Status::BadRequest,
            Json(serde_json::json!({"error": "sender parameter is required"})),
        ));
    }

    let conn = db.conn();

    let rows = conn
        .execute(
            "DELETE FROM bookmarks WHERE room_id = ?1 AND sender = ?2",
            params![room_id, sender],
        )
        .map_err(|e| {
            (
                Status::InternalServerError,
                Json(serde_json::json!({"error": format!("Database error: {e}")})),
            )
        })?;

    if rows > 0 {
        events.publish(ChatEvent::RoomUnbookmarked {
            room_id: room_id.to_string(),
            sender: sender.to_string(),
        });
    }

    Ok(Json(serde_json::json!({
        "room_id": room_id,
        "sender": sender,
        "bookmarked": false,
        "removed": rows > 0
    })))
}

/// GET /api/v1/bookmarks?sender=<sender> — List sender's bookmarked rooms
#[get("/api/v1/bookmarks?<sender>")]
pub fn list_bookmarks(
    db: &State<Db>,
    sender: &str,
) -> Result<Json<BookmarksResponse>, (Status, Json<serde_json::Value>)> {
    let sender = sender.trim();
    if sender.is_empty() {
        return Err((
            Status::BadRequest,
            Json(serde_json::json!({"error": "sender parameter is required"})),
        ));
    }

    let conn = db.conn();

    let mut stmt = conn
        .prepare(
            "SELECT r.id, r.name, r.description, r.created_at,
                    b.created_at as bookmarked_at,
                    (SELECT COUNT(*) FROM messages WHERE room_id = r.id) as message_count,
                    (SELECT MAX(created_at) FROM messages WHERE room_id = r.id) as last_activity
             FROM bookmarks b
             JOIN rooms r ON r.id = b.room_id
             WHERE b.sender = ?1
             ORDER BY b.created_at DESC",
        )
        .map_err(|e| {
            (
                Status::InternalServerError,
                Json(serde_json::json!({"error": format!("Database error: {e}")})),
            )
        })?;

    let bookmarks: Vec<BookmarkedRoom> = stmt
        .query_map(params![sender], |row| {
            Ok(BookmarkedRoom {
                room_id: row.get(0)?,
                room_name: row.get(1)?,
                description: row.get(2)?,
                created_at: row.get(3)?,
                bookmarked_at: row.get(4)?,
                message_count: row.get(5)?,
                last_activity: row.get(6)?,
            })
        })
        .map_err(|e| {
            (
                Status::InternalServerError,
                Json(serde_json::json!({"error": format!("Database error: {e}")})),
            )
        })?
        .filter_map(|r| r.ok())
        .collect();

    let count = bookmarks.len();

    Ok(Json(BookmarksResponse {
        sender: sender.to_string(),
        bookmarks,
        count,
    }))
}

use crate::db::{generate_admin_key, Db};
use crate::events::{ChatEvent, EventBus};
use crate::models::*;
use crate::rate_limit::{RateLimitConfig, RateLimited, RateLimiter};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{delete, get, post, put, State};
use rusqlite::params;

use super::{AdminKey, ClientIp};

#[post("/api/v1/rooms", format = "json", data = "<body>")]
pub fn create_room(
    db: &State<Db>,
    rate_limiter: &State<RateLimiter>,
    rate_config: &State<RateLimitConfig>,
    ip: ClientIp,
    body: Json<CreateRoom>,
) -> Result<RateLimited<serde_json::Value>, (Status, Json<serde_json::Value>)> {
    let rl = rate_limiter.check_with_info(&format!("create_room:{}", ip.0), rate_config.rooms_max, rate_config.rooms_window_secs);
    if !rl.allowed {
        return Err((
            Status::TooManyRequests,
            Json(serde_json::json!({
                "error": format!("Rate limited: max {} rooms per hour", rate_config.rooms_max),
                "retry_after_secs": rl.retry_after_secs,
                "limit": rl.limit,
                "remaining": 0
            })),
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
        Ok(_) => Ok(RateLimited::new(Json(serde_json::json!({
            "id": id,
            "name": name,
            "description": body.description,
            "created_by": body.created_by,
            "admin_key": admin_key,
            "created_at": now,
            "updated_at": now
        })), rl)),
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

#[get("/api/v1/rooms?<include_archived>&<sender>")]
pub fn list_rooms(db: &State<Db>, include_archived: Option<bool>, sender: Option<&str>) -> Json<Vec<RoomWithStats>> {
    let conn = db.conn.lock().unwrap();
    let include = include_archived.unwrap_or(false);

    // When sender is provided, include bookmark status and sort bookmarked rooms first
    if let Some(sender_val) = sender {
        let sender_val = sender_val.trim();
        if !sender_val.is_empty() {
            let sql = if include {
                "SELECT r.id, r.name, r.description, r.created_by, r.created_at, r.updated_at,
                        (SELECT COUNT(*) FROM messages WHERE room_id = r.id) as message_count,
                        (SELECT MAX(created_at) FROM messages WHERE room_id = r.id) as last_activity,
                        (SELECT sender FROM messages WHERE room_id = r.id ORDER BY seq DESC LIMIT 1) as last_sender,
                        (SELECT SUBSTR(content, 1, 100) FROM messages WHERE room_id = r.id ORDER BY seq DESC LIMIT 1) as last_preview,
                        r.archived_at,
                        (SELECT 1 FROM bookmarks WHERE room_id = r.id AND sender = ?1) as is_bookmarked
                 FROM rooms r WHERE COALESCE(r.room_type, 'room') != 'dm'
                 ORDER BY is_bookmarked IS NOT NULL DESC, last_activity IS NULL, last_activity DESC, r.name"
            } else {
                "SELECT r.id, r.name, r.description, r.created_by, r.created_at, r.updated_at,
                        (SELECT COUNT(*) FROM messages WHERE room_id = r.id) as message_count,
                        (SELECT MAX(created_at) FROM messages WHERE room_id = r.id) as last_activity,
                        (SELECT sender FROM messages WHERE room_id = r.id ORDER BY seq DESC LIMIT 1) as last_sender,
                        (SELECT SUBSTR(content, 1, 100) FROM messages WHERE room_id = r.id ORDER BY seq DESC LIMIT 1) as last_preview,
                        r.archived_at,
                        (SELECT 1 FROM bookmarks WHERE room_id = r.id AND sender = ?1) as is_bookmarked
                 FROM rooms r WHERE COALESCE(r.room_type, 'room') != 'dm' AND r.archived_at IS NULL
                 ORDER BY is_bookmarked IS NOT NULL DESC, last_activity IS NULL, last_activity DESC, r.name"
            };
            let mut stmt = conn.prepare(sql).unwrap();
            let rooms = stmt
                .query_map(params![sender_val], |row| {
                    let is_bookmarked: Option<i64> = row.get(11)?;
                    Ok(RoomWithStats {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        description: row.get(2)?,
                        created_by: row.get(3)?,
                        created_at: row.get(4)?,
                        updated_at: row.get(5)?,
                        message_count: row.get(6)?,
                        last_activity: row.get(7)?,
                        last_message_sender: row.get(8)?,
                        last_message_preview: row.get(9)?,
                        archived_at: row.get(10)?,
                        bookmarked: Some(is_bookmarked.is_some()),
                    })
                })
                .unwrap()
                .filter_map(|r| r.ok())
                .collect();
            return Json(rooms);
        }
    }

    let sql = if include {
        "SELECT r.id, r.name, r.description, r.created_by, r.created_at, r.updated_at,
                (SELECT COUNT(*) FROM messages WHERE room_id = r.id) as message_count,
                (SELECT MAX(created_at) FROM messages WHERE room_id = r.id) as last_activity,
                (SELECT sender FROM messages WHERE room_id = r.id ORDER BY seq DESC LIMIT 1) as last_sender,
                (SELECT SUBSTR(content, 1, 100) FROM messages WHERE room_id = r.id ORDER BY seq DESC LIMIT 1) as last_preview,
                r.archived_at
         FROM rooms r WHERE COALESCE(r.room_type, 'room') != 'dm' ORDER BY last_activity IS NULL, last_activity DESC, r.name"
    } else {
        "SELECT r.id, r.name, r.description, r.created_by, r.created_at, r.updated_at,
                (SELECT COUNT(*) FROM messages WHERE room_id = r.id) as message_count,
                (SELECT MAX(created_at) FROM messages WHERE room_id = r.id) as last_activity,
                (SELECT sender FROM messages WHERE room_id = r.id ORDER BY seq DESC LIMIT 1) as last_sender,
                (SELECT SUBSTR(content, 1, 100) FROM messages WHERE room_id = r.id ORDER BY seq DESC LIMIT 1) as last_preview,
                r.archived_at
         FROM rooms r WHERE COALESCE(r.room_type, 'room') != 'dm' AND r.archived_at IS NULL ORDER BY last_activity IS NULL, last_activity DESC, r.name"
    };
    let mut stmt = conn.prepare(sql).unwrap();
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
                last_message_sender: row.get(8)?,
                last_message_preview: row.get(9)?,
                archived_at: row.get(10)?,
                bookmarked: None,
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
                (SELECT MAX(created_at) FROM messages WHERE room_id = r.id) as last_activity,
                (SELECT sender FROM messages WHERE room_id = r.id ORDER BY seq DESC LIMIT 1) as last_sender,
                (SELECT SUBSTR(content, 1, 100) FROM messages WHERE room_id = r.id ORDER BY seq DESC LIMIT 1) as last_preview,
                r.archived_at
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
                last_message_sender: row.get(8)?,
                last_message_preview: row.get(9)?,
                archived_at: row.get(10)?,
                bookmarked: None,
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

#[put("/api/v1/rooms/<room_id>", format = "json", data = "<body>")]
pub fn update_room(
    db: &State<Db>,
    events: &State<EventBus>,
    room_id: &str,
    admin: AdminKey,
    body: Json<UpdateRoom>,
) -> Result<Json<RoomWithStats>, (Status, Json<serde_json::Value>)> {
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

    // Validate name if provided
    if let Some(ref name) = body.name {
        let trimmed = name.trim();
        if trimmed.is_empty() || trimmed.len() > 100 {
            return Err((
                Status::BadRequest,
                Json(serde_json::json!({"error": "Room name must be 1-100 characters"})),
            ));
        }
    }

    // Build dynamic UPDATE
    let now = chrono::Utc::now().to_rfc3339();
    let mut updates: Vec<String> = vec!["updated_at = ?1".to_string()];
    let mut param_idx = 2;

    if body.name.is_some() {
        updates.push(format!("name = ?{}", param_idx));
        param_idx += 1;
    }
    if body.description.is_some() {
        updates.push(format!("description = ?{}", param_idx));
    }

    // Build params dynamically
    let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(now.clone())];
    if let Some(ref name) = body.name {
        param_values.push(Box::new(name.trim().to_string()));
    }
    if let Some(ref desc) = body.description {
        param_values.push(Box::new(desc.clone()));
    }
    param_values.push(Box::new(room_id.to_string()));

    let final_sql = format!(
        "UPDATE rooms SET {} WHERE id = ?{}",
        updates.join(", "),
        param_values.len()
    );

    let params_refs: Vec<&dyn rusqlite::types::ToSql> =
        param_values.iter().map(|p| p.as_ref()).collect();

    match conn.execute(&final_sql, params_refs.as_slice()) {
        Ok(0) => {
            return Err((
                Status::NotFound,
                Json(serde_json::json!({"error": "Room not found"})),
            ));
        }
        Ok(_) => {}
        Err(e) if e.to_string().contains("UNIQUE") => {
            return Err((
                Status::Conflict,
                Json(serde_json::json!({"error": "A room with that name already exists"})),
            ));
        }
        Err(e) => {
            return Err((
                Status::InternalServerError,
                Json(serde_json::json!({"error": e.to_string()})),
            ));
        }
    }

    // Fetch updated room with stats
    let room = conn
        .query_row(
            "SELECT r.id, r.name, r.description, r.created_by, r.created_at, r.updated_at,
                    (SELECT COUNT(*) FROM messages WHERE room_id = r.id) as message_count,
                    (SELECT MAX(created_at) FROM messages WHERE room_id = r.id) as last_activity,
                    (SELECT sender FROM messages WHERE room_id = r.id ORDER BY seq DESC LIMIT 1) as last_sender,
                    (SELECT SUBSTR(content, 1, 100) FROM messages WHERE room_id = r.id ORDER BY seq DESC LIMIT 1) as last_preview,
                    r.archived_at
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
                    last_message_sender: row.get(8)?,
                    last_message_preview: row.get(9)?,
                    archived_at: row.get(10)?,
                    bookmarked: None,
                })
            },
        )
        .map_err(|_| {
            (
                Status::InternalServerError,
                Json(serde_json::json!({"error": "Failed to fetch updated room"})),
            )
        })?;

    // Publish SSE event
    events.publish(ChatEvent::RoomUpdated(room.clone()));

    Ok(Json(room))
}

#[post("/api/v1/rooms/<room_id>/archive")]
pub fn archive_room(
    db: &State<Db>,
    events: &State<EventBus>,
    room_id: &str,
    admin: AdminKey,
) -> Result<Json<RoomWithStats>, (Status, Json<serde_json::Value>)> {
    let conn = db.conn.lock().unwrap();

    // Verify room exists and admin key matches
    let row: (Option<String>, Option<String>) = conn
        .query_row(
            "SELECT admin_key, archived_at FROM rooms WHERE id = ?1",
            params![room_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|_| {
            (
                Status::NotFound,
                Json(serde_json::json!({"error": "Room not found"})),
            )
        })?;

    match row.0 {
        Some(ref key) if key == &admin.0 => {}
        _ => {
            return Err((
                Status::Forbidden,
                Json(serde_json::json!({"error": "Invalid admin key for this room"})),
            ));
        }
    }

    // Check if already archived
    if row.1.is_some() {
        return Err((
            Status::Conflict,
            Json(serde_json::json!({"error": "Room is already archived"})),
        ));
    }

    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE rooms SET archived_at = ?1, updated_at = ?1 WHERE id = ?2",
        params![&now, room_id],
    )
    .map_err(|e| {
        (
            Status::InternalServerError,
            Json(serde_json::json!({"error": e.to_string()})),
        )
    })?;

    // Fetch updated room
    let room = conn
        .query_row(
            "SELECT r.id, r.name, r.description, r.created_by, r.created_at, r.updated_at,
                    (SELECT COUNT(*) FROM messages WHERE room_id = r.id) as message_count,
                    (SELECT MAX(created_at) FROM messages WHERE room_id = r.id) as last_activity,
                    (SELECT sender FROM messages WHERE room_id = r.id ORDER BY seq DESC LIMIT 1) as last_sender,
                    (SELECT SUBSTR(content, 1, 100) FROM messages WHERE room_id = r.id ORDER BY seq DESC LIMIT 1) as last_preview,
                    r.archived_at
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
                    last_message_sender: row.get(8)?,
                    last_message_preview: row.get(9)?,
                    archived_at: row.get(10)?,
                    bookmarked: None,
                })
            },
        )
        .map_err(|_| {
            (
                Status::InternalServerError,
                Json(serde_json::json!({"error": "Failed to fetch room"})),
            )
        })?;

    events.publish(ChatEvent::RoomArchived(room.clone()));

    Ok(Json(room))
}

#[post("/api/v1/rooms/<room_id>/unarchive")]
pub fn unarchive_room(
    db: &State<Db>,
    events: &State<EventBus>,
    room_id: &str,
    admin: AdminKey,
) -> Result<Json<RoomWithStats>, (Status, Json<serde_json::Value>)> {
    let conn = db.conn.lock().unwrap();

    // Verify room exists and admin key matches
    let row: (Option<String>, Option<String>) = conn
        .query_row(
            "SELECT admin_key, archived_at FROM rooms WHERE id = ?1",
            params![room_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|_| {
            (
                Status::NotFound,
                Json(serde_json::json!({"error": "Room not found"})),
            )
        })?;

    match row.0 {
        Some(ref key) if key == &admin.0 => {}
        _ => {
            return Err((
                Status::Forbidden,
                Json(serde_json::json!({"error": "Invalid admin key for this room"})),
            ));
        }
    }

    // Check if not archived
    if row.1.is_none() {
        return Err((
            Status::Conflict,
            Json(serde_json::json!({"error": "Room is not archived"})),
        ));
    }

    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE rooms SET archived_at = NULL, updated_at = ?1 WHERE id = ?2",
        params![&now, room_id],
    )
    .map_err(|e| {
        (
            Status::InternalServerError,
            Json(serde_json::json!({"error": e.to_string()})),
        )
    })?;

    // Fetch updated room
    let room = conn
        .query_row(
            "SELECT r.id, r.name, r.description, r.created_by, r.created_at, r.updated_at,
                    (SELECT COUNT(*) FROM messages WHERE room_id = r.id) as message_count,
                    (SELECT MAX(created_at) FROM messages WHERE room_id = r.id) as last_activity,
                    (SELECT sender FROM messages WHERE room_id = r.id ORDER BY seq DESC LIMIT 1) as last_sender,
                    (SELECT SUBSTR(content, 1, 100) FROM messages WHERE room_id = r.id ORDER BY seq DESC LIMIT 1) as last_preview,
                    r.archived_at
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
                    last_message_sender: row.get(8)?,
                    last_message_preview: row.get(9)?,
                    archived_at: row.get(10)?,
                    bookmarked: None,
                })
            },
        )
        .map_err(|_| {
            (
                Status::InternalServerError,
                Json(serde_json::json!({"error": "Failed to fetch room"})),
            )
        })?;

    events.publish(ChatEvent::RoomUnarchived(room.clone()));

    Ok(Json(room))
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

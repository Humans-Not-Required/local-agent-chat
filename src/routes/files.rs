use crate::db::Db;
use crate::events::{ChatEvent, EventBus};
use crate::models::*;
use crate::rate_limit::RateLimiter;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{delete, get, post, State};
use rusqlite::params;

use super::{AdminKey, ClientIp};

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

    let rl = rate_limiter.check_with_info(&format!("upload_file:{}", ip.0), 10, 60);
    if !rl.allowed {
        return Err((
            Status::TooManyRequests,
            Json(serde_json::json!({
                "error": "Rate limited: max 10 file uploads per minute",
                "retry_after_secs": rl.retry_after_secs,
                "limit": rl.limit,
                "remaining": 0
            })),
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
            Json(
                serde_json::json!({"error": format!("File too large: {} bytes (max {} bytes)", decoded.len(), MAX_FILE_SIZE)}),
            ),
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
                Json(serde_json::json!({"error": "Sender query parameter required (or use room admin key)"})),
            )
        })?;
        if sender != existing_sender {
            return Err((
                Status::Forbidden,
                Json(
                    serde_json::json!({"error": "Only the original uploader can delete this file"}),
                ),
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

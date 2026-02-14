use rocket::serde::json::Json;
use rocket::{get, put, State};
use rocket::http::Status;
use rusqlite::params;

use crate::db::Db;
use crate::events::{ChatEvent, EventBus};
use crate::models::{ReadPosition, UnreadInfo, UnreadResponse, UpdateReadPosition};

/// PUT /api/v1/rooms/<room_id>/read — Mark room as read up to a seq number.
/// Upserts the read position for the given sender.
#[put("/api/v1/rooms/<room_id>/read", data = "<body>")]
pub fn update_read_position(
    room_id: &str,
    body: Json<UpdateReadPosition>,
    db: &State<Db>,
    events: &State<EventBus>,
) -> Result<Json<ReadPosition>, Status> {
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
        return Err(Status::NotFound);
    }

    // Validate sender
    let sender = body.sender.trim();
    if sender.is_empty() || sender.len() > 100 {
        return Err(Status::BadRequest);
    }

    // Validate last_read_seq is positive
    if body.last_read_seq < 0 {
        return Err(Status::BadRequest);
    }

    let now = chrono::Utc::now().to_rfc3339();

    // UPSERT: insert or update if the new seq is higher
    conn.execute(
        "INSERT INTO read_positions (room_id, sender, last_read_seq, updated_at)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(room_id, sender) DO UPDATE SET
           last_read_seq = MAX(read_positions.last_read_seq, excluded.last_read_seq),
           updated_at = excluded.updated_at
         WHERE excluded.last_read_seq > read_positions.last_read_seq",
        params![room_id, sender, body.last_read_seq, &now],
    )
    .map_err(|_| Status::InternalServerError)?;

    // Read back the actual value (might not have changed if new seq was lower)
    let position = conn
        .query_row(
            "SELECT room_id, sender, last_read_seq, updated_at FROM read_positions WHERE room_id = ?1 AND sender = ?2",
            params![room_id, sender],
            |row| {
                Ok(ReadPosition {
                    room_id: row.get(0)?,
                    sender: row.get(1)?,
                    last_read_seq: row.get(2)?,
                    updated_at: row.get(3)?,
                })
            },
        )
        .map_err(|_| Status::InternalServerError)?;

    // Broadcast the read position update
    events.publish(ChatEvent::ReadPositionUpdated(position.clone()));

    Ok(Json(position))
}

/// GET /api/v1/rooms/<room_id>/read — Get all read positions for a room.
#[get("/api/v1/rooms/<room_id>/read")]
pub fn get_read_positions(room_id: &str, db: &State<Db>) -> Result<Json<Vec<ReadPosition>>, Status> {
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
        return Err(Status::NotFound);
    }

    let mut stmt = conn
        .prepare(
            "SELECT room_id, sender, last_read_seq, updated_at FROM read_positions WHERE room_id = ?1 ORDER BY updated_at DESC",
        )
        .map_err(|_| Status::InternalServerError)?;

    let positions: Vec<ReadPosition> = stmt
        .query_map(params![room_id], |row| {
            Ok(ReadPosition {
                room_id: row.get(0)?,
                sender: row.get(1)?,
                last_read_seq: row.get(2)?,
                updated_at: row.get(3)?,
            })
        })
        .map_err(|_| Status::InternalServerError)?
        .filter_map(|r| r.ok())
        .collect();

    Ok(Json(positions))
}

/// GET /api/v1/unread?sender=<name> — Get unread counts across all rooms for a sender.
#[get("/api/v1/unread?<sender>")]
pub fn get_unread(sender: &str, db: &State<Db>) -> Result<Json<UnreadResponse>, Status> {
    let sender = sender.trim();
    if sender.is_empty() {
        return Err(Status::BadRequest);
    }

    let conn = db.conn.lock().unwrap();

    // Get all rooms with their latest seq, unread count, and the sender's read position.
    // Uses COUNT to compute unread (seq is global, not per-room, so arithmetic won't work).
    let mut stmt = conn
        .prepare(
            "SELECT r.id, r.name,
                    COALESCE(MAX(m.seq), 0) as latest_seq,
                    COALESCE(rp.last_read_seq, 0) as last_read_seq,
                    COUNT(CASE WHEN m.seq > COALESCE(rp.last_read_seq, 0) THEN 1 END) as unread_count
             FROM rooms r
             LEFT JOIN messages m ON m.room_id = r.id
             LEFT JOIN read_positions rp ON rp.room_id = r.id AND rp.sender = ?1
             GROUP BY r.id
             ORDER BY r.name",
        )
        .map_err(|_| Status::InternalServerError)?;

    let rooms: Vec<UnreadInfo> = stmt
        .query_map(params![sender], |row| {
            Ok(UnreadInfo {
                room_id: row.get(0)?,
                room_name: row.get(1)?,
                unread_count: row.get(4)?,
                last_read_seq: row.get(3)?,
                latest_seq: row.get(2)?,
            })
        })
        .map_err(|_| Status::InternalServerError)?
        .filter_map(|r| r.ok())
        .collect();

    let total_unread: i64 = rooms.iter().map(|r| r.unread_count).sum();

    Ok(Json(UnreadResponse {
        sender: sender.to_string(),
        rooms,
        total_unread,
    }))
}

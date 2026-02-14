use crate::db::Db;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{get, State};
use rusqlite::params;

#[get("/api/v1/rooms/<room_id>/participants")]
pub fn room_participants(
    db: &State<Db>,
    room_id: &str,
) -> Result<Json<Vec<crate::models::Participant>>, (Status, Json<serde_json::Value>)> {
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

    // Aggregate participants from messages in this room.
    // Use the most recent sender_type for each sender.
    let mut stmt = conn
        .prepare(
            "SELECT sender,
                    (SELECT m2.sender_type FROM messages m2 WHERE m2.room_id = ?1 AND m2.sender = m.sender AND m2.sender_type IS NOT NULL ORDER BY m2.seq DESC LIMIT 1) as latest_sender_type,
                    COUNT(*) as message_count,
                    MIN(created_at) as first_seen,
                    MAX(created_at) as last_seen
             FROM messages m
             WHERE room_id = ?1
             GROUP BY sender
             ORDER BY last_seen DESC",
        )
        .map_err(|e| {
            (
                Status::InternalServerError,
                Json(serde_json::json!({"error": e.to_string()})),
            )
        })?;

    let participants = stmt
        .query_map(params![room_id], |row| {
            Ok(crate::models::Participant {
                sender: row.get(0)?,
                sender_type: row.get(1)?,
                message_count: row.get(2)?,
                first_seen: row.get(3)?,
                last_seen: row.get(4)?,
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

    Ok(Json(participants))
}
